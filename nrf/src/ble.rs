use crate::{
    command_writer::WriterQueue,
    fmt::{error, info, unwrap},
};
use common::{bundles::ToHeadlightBundle, types::*};
use core::mem;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use nrf_softdevice::{
    ble::{gatt_server, peripheral as ble_peripheral, Connection, Phy},
    generate_adv_data, raw, Softdevice,
};
use static_cell::StaticCell;
use tiny_serde::{prelude::*, Deserialize, Serialize};

const ADV_DATA: &[u8] = generate_adv_data! {
    flags: (LE_Only, GeneralDiscovery),
    services: Complete128(Custom("0b2adcf1-38a7-48f9-a61d-8311fe471b70")),
    short_name: "HLV2"
};

const SCAN_DATA: &[u8] = generate_adv_data! {
    full_name: "Headlights V2"
};

const ADV: ble_peripheral::ConnectableAdvertisement<'static> =
    ble_peripheral::ConnectableAdvertisement::ScannableUndirected {
        adv_data: ADV_DATA,
        scan_data: SCAN_DATA,
    };

static MODEL: StaticCell<BLE> = StaticCell::new();

#[nrf_softdevice::gatt_service(uuid = "0b2adcf1-38a7-48f9-a61d-8311fe471b70")]
pub struct HeadlightService {
    #[characteristic(uuid = "9a00bcc5-89f1-4b9d-88bd-f2033440a5b4", write)]
    pub request: [u8; <Request as _TinyDeSized>::SIZE],

    // responses
    #[characteristic(uuid = "ccf82e46-5f1c-4671-b481-7ffd2854fed4", notify)]
    pub status: [u8; <Status as _TinyDeSized>::SIZE],

    #[characteristic(uuid = "eb483eeb-7b8e-45e0-910b-6c88fb3d75f3", write, notify)]
    pub control: [u8; <Control as _TinyDeSized>::SIZE],

    #[characteristic(uuid = "30f62c01-d9d8-4c14-9a66-36ad0d92edbf", notify)]
    pub monitor: [u8; <Monitor as _TinyDeSized>::SIZE],

    #[characteristic(uuid = "73e4b52c-4ae2-4901-b78b-8f95f3a60cdb", write, notify)]
    pub config: [u8; <Config as _TinyDeSized>::SIZE],

    // diagnostic
    #[characteristic(uuid = "a16bc310-eb50-414e-87b3-2199e79523c2", notify)]
    pub app_error: [u8; <AppErrorData as _TinyDeSized>::SIZE],
}

#[nrf_softdevice::gatt_server]
pub struct Server {
    pub headlight: HeadlightService,
}

pub struct BLE {
    sd: &'static Softdevice,
    conn: Mutex<ThreadModeRawMutex, Option<Connection>>,
    server: Server,
}

impl BLE {
    const fn new(sd: &'static Softdevice, server: Server) -> Self {
        Self {
            sd,
            conn: Mutex::new(None),
            server,
        }
    }

    pub async fn init(spawner: &Spawner) -> &'static Self {
        let sd_config = nrf_softdevice::Config {
            conn_gap: Some(raw::ble_gap_conn_cfg_t {
                conn_count: raw::BLE_GAP_CONN_COUNT_DEFAULT as u8,
                event_length: raw::BLE_GAP_EVENT_LENGTH_DEFAULT as u16,
            }),
            gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
                p_value: b"Headlights V2" as *const u8 as _,
                current_len: 13,
                max_len: 13,
                write_perm: unsafe { mem::zeroed() },
                _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                    raw::BLE_GATTS_VLOC_STACK as u8,
                ),
            }),
            conn_gatt: Some(raw::ble_gatt_conn_cfg_t {
                att_mtu: raw::BLE_GATT_ATT_MTU_DEFAULT as u16,
            }),
            common_vs_uuid: Some(raw::ble_common_cfg_vs_uuid_t {
                vs_uuid_count: raw::BLE_UUID_VS_COUNT_DEFAULT as u8,
            }),
            ..Default::default()
        };

        let sd = Softdevice::enable(&sd_config);

        let server = unwrap!(Server::new(sd));

        spawner.must_spawn(softdevice_task(sd));

        MODEL.init(BLE::new(sd, server))
    }

    pub async fn get_conn(&self) -> Option<Connection> {
        let lock = self.conn.lock().await;
        lock.clone()
    }

    pub async fn set_conn(&self, conn: Option<Connection>) {
        let mut lock = self.conn.lock().await;
        *lock = conn;
    }

    pub fn get_server(&self) -> &Server {
        &self.server
    }

    pub async fn run(&self, queue: &'static WriterQueue) -> ! {
        let adv_config = ble_peripheral::Config {
            primary_phy: Phy::M1,
            secondary_phy: Phy::M1,
            ..Default::default()
        };

        loop {
            let conn =
                unwrap!(ble_peripheral::advertise_connectable(self.sd, ADV, &adv_config).await);

            self.set_conn(Some(conn.clone())).await;

            info!("advertising done!");

            let e = gatt_server::run(&conn, &self.server, |e| match e {
                ServerEvent::Headlight(e) => {
                    let bundle: Option<ToHeadlightBundle> = match e {
                        HeadlightServiceEvent::RequestWrite(data) => {
                            Request::deserialize(data).map(Request::into)
                        }
                        HeadlightServiceEvent::ControlWrite(data) => {
                            Control::deserialize(data).map(Control::into)
                        }
                        HeadlightServiceEvent::ConfigWrite(data) => {
                            Config::deserialize(data).map(Config::into)
                        }
                        _ => return
                    };

                    if let Some(bundle) = bundle {
                        if queue.try_send(bundle).is_err() { // only possible error is it's full
                            error!("Command ingestion channel overflowed (commands are being received faster than they can be dispatched).");
                            self.server.headlight.app_error_notify(&conn, &AppErrorData::TooFast.serialize()).ok();
                        }
                    } else {
                        error!("Invalid BLE packet received (command could not be serialized from received bytes).");
                        self.server
                            .headlight
                            .app_error_notify(&conn, &AppErrorData::InvalidPacket.serialize())
                            .ok();
                    }
                }
            })
            .await;

            self.set_conn(None).await;

            info!("gatt_server run exited with error: {:?}", e);
        }
    }
}

#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}
