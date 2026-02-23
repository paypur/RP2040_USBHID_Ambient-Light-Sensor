import usb_hid

# HID Usage Tables: 1.6.0
# Descriptor size: 108 (bytes)
# +----------+---------+-------------------+
# | ReportId | Kind    | ReportSizeInBytes |
# +----------+---------+-------------------+
# |        1 | Input   |                 3 |
# +----------+---------+-------------------+
# |        2 | Feature |                 3 |
# +----------+---------+-------------------+

'''
Ref:
www.usb.org/sites/default/files/hutrr39b_0.pdf
www.usb.org/sites/default/files/hut1_4.pdf

ReportID: 1 (Input Report)

4D1 - Data Field: Illuminance: 0 - 65535 (unit: Lux) - 16 bits
202 - Event: Sensor Event: 4 (Data Updated) - 3 bits

Packing:
Byte 0 | 0-7 Illuminance (bits 0-7) |
Byte 1 | 0-7 Illuminance (bits 8-15)|
Byte 2 | 0-2 Sensor Event | Padding |


ReportID: 2 (Feature Report)

Supported Features and responses:
316 - Property: Reporting State: - 2 bits
319 - Property: Power State: - 3 bits
30E - Property: Report Interval (unit: ms) - 12 bits
'''


HID_REPORT_DESCRIPTOR = bytes([
    0x05, 0x20,                      # UsagePage(Sensors[0x0020])
    0x09, 0x01,                      # UsageId(Sensor[0x0001])
    0xA1, 0x01,                      # Collection(Application)
    0x09, 0x41,                      #     UsageId(Light: Ambient Light[0x0041])
    0xA1, 0x00,                      #     Collection(Physical)
    0x85, 0x01,                      #         ReportId(1)
    0x0A, 0xD1, 0x04,                #         UsageId(Data Field: Illuminance[0x04D1])
    0x15, 0x00,                      #         LogicalMinimum(0)
    0x27, 0xFF, 0xFF, 0x00, 0x00,    #         LogicalMaximum(65,535)
    0x95, 0x01,                      #         ReportCount(1)
    0x75, 0x10,                      #         ReportSize(16)
    0x81, 0x02,                      #         Input(Data, Variable, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, BitField)
    0x0A, 0x02, 0x02,                #         UsageId(Event: Sensor Event[0x0202])
    0xA1, 0x02,                      #         Collection(Logical)
    0x1A, 0x10, 0x08,                #             UsageIdMin(Sensor Event: Unknown[0x0810])
    0x2A, 0x15, 0x08,                #             UsageIdMax(Sensor Event: Change Sensitivity[0x0815])
    0x15, 0x01,                      #             LogicalMinimum(1)
    0x25, 0x06,                      #             LogicalMaximum(6)
    0x75, 0x03,                      #             ReportSize(3)
    0x81, 0x00,                      #             Input(Data, Array, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, BitField)
    0xC0,                            #         EndCollection()
    0x75, 0x05,                      #         ReportSize(5)
    0x81, 0x03,                      #         Input(Constant, Variable, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, BitField)
    0x85, 0x02,                      #         ReportId(2)
    0x0A, 0x16, 0x03,                #         UsageId(Property: Reporting State[0x0316])
    0xA1, 0x02,                      #         Collection(Logical)
    0x1A, 0x40, 0x08,                #             UsageIdMin(Reporting State: Report No Events[0x0840])
    0x2A, 0x41, 0x08,                #             UsageIdMax(Reporting State: Report All Events[0x0841])
    0x25, 0x02,                      #             LogicalMaximum(2)
    0x75, 0x02,                      #             ReportSize(2)
    0xB1, 0x00,                      #             Feature(Data, Array, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, NonVolatile, BitField)
    0xC0,                            #         EndCollection()
    0x0A, 0x19, 0x03,                #         UsageId(Property: Power State[0x0319])
    0xA1, 0x02,                      #         Collection(Logical)
    0x1A, 0x50, 0x08,                #             UsageIdMin(Power State: Undefined[0x0850])
    0x2A, 0x55, 0x08,                #             UsageIdMax(Power State: D4 Power Off[0x0855])
    0x25, 0x06,                      #             LogicalMaximum(6)
    0x75, 0x03,                      #             ReportSize(3)
    0xB1, 0x00,                      #             Feature(Data, Array, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, NonVolatile, BitField)
    0xC0,                            #         EndCollection()
    0x0A, 0x0E, 0x03,                #         UsageId(Property: Report Interval[0x030E])
    0x15, 0x00,                      #         LogicalMinimum(0)
    0x26, 0xFF, 0x0F,                #         LogicalMaximum(4,095)
    0x75, 0x0C,                      #         ReportSize(12)
    0xB1, 0x02,                      #         Feature(Data, Variable, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, NonVolatile, BitField)
    0x75, 0x07,                      #         ReportSize(7)
    0xB1, 0x03,                      #         Feature(Constant, Variable, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, NonVolatile, BitField)
    0xC0,                            #     EndCollection()
    0xC0,                            # EndCollection()
])

als_device = usb_hid.Device(
    report_descriptor=HID_REPORT_DESCRIPTOR,
    usage_page=0x20,           # Sensors
    usage=0x01,                # Sensor
    report_ids=(1,2),             # Report ID 1 (Input Report) and 2 (Feature Report)
    in_report_lengths=(3,3),
    out_report_lengths=(0,3),
)

usb_hid.enable((als_device,))

