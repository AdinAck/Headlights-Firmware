from rich import print
from serial.tools.list_ports import comports
from serial import Serial
import crc

from random import randbytes
from time import sleep

PORT: str = [port for d in comports() if 'cu.usbserial' in (port := d.device)][0]
CALCULATOR = crc.Calculator(crc.Crc8.AUTOSAR) # type: ignore

def read_bytes_forever(ser: Serial):
    while ...:
        print(f'0x{ser.read(1).hex()}')

def crcify(b: bytes, i: int) -> bytes:
    return b[:i] + bytes([CALCULATOR.checksum(b)]) + b[i:]

def bulk_write(ser: Serial):
    
    while ...:
        # ser.write(crcify(bytes([0x1f, 0xf0, 0x00]), 1)) # status
        # sleep(0.01)
        # ser.write(crcify(bytes([0xaa]) + randbytes(1), 1)) # brightness
        # sleep(0.01)
        # ser.write(crcify(bytes([0xab]) + randbytes(3), 1)) # monitor
        # sleep(0.01)
        # ser.write(crcify(bytes([0xac]) + randbytes(2) + bytes([0xde]) + randbytes(2), 1)) # pid
        # sleep(0.01)
        
        send = (
            crcify(bytes([0x1f, 0xf0, 0x00]), 1) + # status
            crcify(bytes([0xaa]) + randbytes(1), 1) + # brightness
            crcify(bytes([0xab]) + randbytes(3), 1) + # monitor
            crcify(bytes([0xac]) + randbytes(2) + bytes([0xde]) + randbytes(2), 1)
        )
        
        ser.write(send)
        sleep(0.05)
        

if __name__ == '__main__':
    with Serial(PORT, 9600) as ser:
        # ser.write(bytes([0] * 10)) # pad buffer with noop
        
        # ser.write(crcify(bytes([0x1f, 0xf0, 0x00]), 1)) # status
        # sleep(2)
        # ser.write(crcify(bytes([0xaa, 0x0f]), 1)) # brightness
        # sleep(2)
        # ser.write(crcify(bytes([0xab, 0x00, 0x00, 0x00]), 1)) # monitor
        # sleep(2)
        # ser.write(crcify(bytes([0xac, 0x00, 0x00, 0x00, 0x00, 0x00]), 1))
        # sleep(2)
        
        send = (
            bytes([0xee, 0x00]) + # nonsense
            crcify(bytes([0x1f, 0xf0, 0x00]), 1) + # status
            crcify(bytes([0xaa, 0x0f]), 1) + # brightness
            crcify(bytes([0xab, 0x00, 0x00, 0x00]), 1) + # monitor
            crcify(bytes([0xac, 0x00, 0x00, 0x00, 0x00, 0x00]), 1)
        )

        print([i for i in send])
        ser.write(send)
        
        bulk_write(ser)