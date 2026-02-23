import usb_hid
import time
import board
import analogio
from digitalio import DigitalInOut, Direction
from neopixel import NeoPixel



# Global Variables
POWER_STATE = 6    # 1: Undefined 2: Full Power (D0), 3: Low Power (D1), 4: Standby Power (D2) 5: Sleep (D3) 6: Power Off (D4)
REPORTING_STATE = 1    # 1: Report No Events, 2: Report All events
REPORT_INTERVAL = 100   # ms
SENSOR_EVENT = 4           # Data updated

als_device = usb_hid.devices[0]

# Configure pins per spec:
# Pin 28 must be HIGH, Pin 27 must be LOW
pin28 = DigitalInOut(board.GP28)
pin28.direction = Direction.OUTPUT
pin28.value = True

pin27 = DigitalInOut(board.GP27)
pin27.direction = Direction.OUTPUT
pin27.value = False

# ADC on GP26
adc = analogio.AnalogIn(board.GP26)

# Waveshare RP2040-Zero Only
enable_led = True
pixel = NeoPixel(board.GP16, 1)

def status_led():
    global pixel
    if enable_led and POWER_STATE == 6:
        # Power Off
        pixel[0] = (0,0,2) # Blue
        pixel.write()
        return
    if enable_led and (POWER_STATE == 2):
        # D0 state
        pixel[0] = (0,2,0) # Green
        pixel.write()
        return
    if enable_led:
        pixel[0] = (2,0,0) # Red
        pixel.write()
    
    return

def send_input_report(illuminance_lux):
    data = (illuminance_lux & 0xffff) | ((SENSOR_EVENT & 0b111) << 16)
    report = data.to_bytes(3,'little')
    print('Sending Input Report:  ',end="")
    print (' '.join(f'{byte:08b}' for byte in reversed(report)))
    
    try:
        als_device.send_report(report,1)
        return True
    except OSError as e:
        print('Output Buffer full')
        return False


def send_feature_report():
    # Send an updated feature report to the host
    # Tip: Use /sys/kernel/debug/hid/0003\:2E8A\:*/rdesc
    # to figure out the packing and offsets
    
    data = (REPORTING_STATE & 0b11) | ((POWER_STATE & 0b111) << 2) | ((REPORT_INTERVAL & 0xfff) << 5)
    
    report = data.to_bytes(3,'little')
    print('Sending Feature Report:  ',end="")
    print (' '.join(f'{byte:08b}' for byte in reversed(report)))

    try:
        als_device.send_report(report,2)
        return True
    except OSError as e:
        print('Output Buffer full')
        return False

def read_feature_report(report):
    
    global REPORTING_STATE
    global POWER_STATE
    global REPORT_INTERVAL
    
    data = int.from_bytes(report,'little')
    
    received_reporting_state = (data & 0b11)
    received_power_state = (data >> 2) & 0b111
    received_report_interval = (data >> 5) & 0xfff

    if received_reporting_state != 0:
        REPORTING_STATE = received_reporting_state
    
    if received_power_state != 0:
        POWER_STATE = received_power_state
    
    if received_report_interval !=0:
        REPORT_INTERVAL = received_report_interval
        
    return


status_led()

# Make sure the output buffers are not empty
# Otherwise, circuitpython sends garbage data out
send_feature_report()
illuminance = int(adc.value)
send_input_report(illuminance)

while True:
    print (f"Power State: {POWER_STATE}, Reporting State: {REPORTING_STATE}, Report Interval: {REPORT_INTERVAL}")
    feature_report_in = als_device.get_last_received_report(2)
    
    if feature_report_in != None:
        print ("Feature report recieved: ",end="")
        print (' '.join(f'{byte:08b}' for byte in reversed(feature_report_in)))
        read_feature_report(feature_report_in)
        #time.sleep(0.1)
        send_feature_report()
        status_led()
        
    if REPORTING_STATE == 1 or POWER_STATE in [1, 4, 5, 6]:
        time.sleep(0.1)
        continue
    
    if REPORTING_STATE == 2 and POWER_STATE in [1, 4, 5, 6]:
        print('Hmmm. This state should never have happened.')
        POWER_STATE = 2
        send_feature_report()

        illuminance = int(adc.value)  # 0-65535 already
        send_input_report(illuminance)
        time.sleep(REPORT_INTERVAL/1000)
        continue
    
    elif REPORTING_STATE == 2:
        illuminance = int(adc.value)
        send_input_report(illuminance)
        time.sleep(REPORT_INTERVAL/1000)
