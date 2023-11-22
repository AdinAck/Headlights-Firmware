/// https://gist.github.com/adamgreig/77296117c3dfa80d3ceecb18640142c9

/// Extract the specified fields from the `Peripherals` struct into several named
/// structs which can be passed to other tasks to provide them with all their
/// resources, including pins, peripherals, DMA channels, etc.
///
/// The `peripherals` module must be in scope when `resource_assigs!{}` is called,
/// and it defines a new macro `split_resources!()` which uses the `Peripherals` struct
/// and returns a new struct with a field for each of the structs named in `resource_assigs!{}`.
///
/// Defines new structs containing the specified structs from the `peripherals` module,
/// a top-level struct that contains an instance of each of these new structs, and a macro
/// that creates the top-level struct and populates it with fields from a `Peripherals` instance.
///
/// # Example
///
/// ```
/// assign_resources! {
///     usb: UsbResources {
///         dp: PA12,
///         dm: PA11,
///         usb: USB,
///     }
///     leds: LedResources {
///         r: PA2,
///         g: PA3,
///         b: PA4,
///         tim2: TIM2,
///     }
/// }
///
/// #[embassy_executor::task]
/// async fn usb_task(r: UsbResources) {
///     // use r.dp, r.dm, r.usb
/// }
///
/// #[embassy_executor::task]
/// async fn led_task(r: LedResources) {
///     // use r.r, r.g, r.b, r.tim2
/// }
///
/// #[embassy_executor::main]
/// async fn main(spawner: embassy_executor::Spawner) {
///     let p = embassy_stm32::init(Default::default());
///     let r = split_resources!(p);
///     spawner.spawn(usb_task(r.usb)).unwrap();
///     spawner.spawn(led_task(r.leds)).unwrap();
///
///     // can still use p.PA0, p.PA1, etc
/// }
/// ```
#[macro_export]
macro_rules! assign_resources {
    {
        $(
            $group_name:ident : $group_struct:ident {
                $($resource_name:ident : $resource_field:ident $(=$ALIAS:ident)?),*
                $(,)?
            }
            $(,)?
        )+
    } => {
        $($($(type $ALIAS = $resource_field;)?)*)*

        #[allow(dead_code,non_snake_case)]
        struct AssignedResources {
            $($group_name : $group_struct),*
        }
        $(
            #[allow(dead_code,non_snake_case)]
            struct $group_struct {
                pub $($resource_name: peripherals::$resource_field),*
            }
        )+
        macro_rules! split_resources (
            ($p:ident) => {
                AssignedResources {
                    $($group_name: $group_struct {
                        $($resource_name: $p.$resource_field),*
                    }),*
                }
            }
        );
    }
}