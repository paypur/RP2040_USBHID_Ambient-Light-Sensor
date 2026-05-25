#include "tusb.h"
#include "pico/unique_id.h"
#include <string.h>
#include <stdio.h>

/* A combination of interfaces must have a unique product id, since PC will save device driver after the first plug.
 */
#define _PID_MAP(itf, n) ((CFG_TUD_##itf) << (n))
#define USB_PID (0x4000 | _PID_MAP(CDC, 0) | _PID_MAP(MSC, 1) | _PID_MAP(HID, 2) | \
                 _PID_MAP(MIDI, 3) | _PID_MAP(VENDOR, 4) )
#define USB_BCD   0x0200

//--------------------------------------------------------------------+
// Device Descriptors
//--------------------------------------------------------------------+
tusb_desc_device_t const desc_device =
{
    .bLength            = sizeof(tusb_desc_device_t),
    .bDescriptorType    = TUSB_DESC_DEVICE,
    .bcdUSB             = 0x0200,
    .bDeviceClass       = 0x00,
    .bDeviceSubClass    = 0x00,
    .bDeviceProtocol    = 0x00,
    .bMaxPacketSize0    = CFG_TUD_ENDPOINT0_SIZE,

    .idVendor           = 0x2E8A,     // Raspberry Pi
    .idProduct          = USB_PID,
    .bcdDevice          = 0x0100,

    .iManufacturer      = 0x01,
    .iProduct           = 0x02,
    .iSerialNumber      = 0x03,

    .bNumConfigurations = 0x01
};

// Invoked when received GET DEVICE DESCRIPTOR
// Application return pointer to descriptor
uint8_t const * tud_descriptor_device_cb(void)
{
  return (uint8_t const *) &desc_device;
}

//--------------------------------------------------------------------+
// HID Report Descriptor
//--------------------------------------------------------------------+

// HID Usage Tables: 1.6.0
uint8_t const desc_hid_report[] =
{
    0x05, 0x20,                      // UsagePage(Sensors[0x0020])
    0x09, 0x01,                      // UsageId(Sensor[0x0001])
    0xA1, 0x01,                      // Collection(Application)
    0x09, 0x41,                      //     UsageId(Light: Ambient Light[0x0041])
    0xA1, 0x00,                      //     Collection(Physical)
    0x85, 0x01,                      //         ReportId(1)
    0x0A, 0xD1, 0x04,                //         UsageId(Data Field: Illuminance[0x04D1])
    0x15, 0x00,                      //         LogicalMinimum(0)
    0x27, 0xFF, 0xFF, 0x00, 0x00,    //         LogicalMaximum(65,535)
    0x95, 0x01,                      //         ReportCount(1)
    0x75, 0x10,                      //         ReportSize(16)
    0x81, 0x02,                      //         Input(Data, Variable, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, BitField)
    0x0A, 0x02, 0x02,                //         UsageId(Event: Sensor Event[0x0202])
    0xA1, 0x02,                      //         Collection(Logical)
    0x1A, 0x10, 0x08,                //             UsageIdMin(Sensor Event: Unknown[0x0810])
    0x2A, 0x15, 0x08,                //             UsageIdMax(Sensor Event: Change Sensitivity[0x0815])
    0x15, 0x01,                      //             LogicalMinimum(1)
    0x25, 0x06,                      //             LogicalMaximum(6)
    0x75, 0x03,                      //             ReportSize(3)
    0x81, 0x00,                      //             Input(Data, Array, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, BitField)
    0xC0,                            //         EndCollection()
    0x75, 0x05,                      //         ReportSize(5)
    0x81, 0x03,                      //         Input(Constant, Variable, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, BitField)
    0x85, 0x02,                      //         ReportId(2)
    0x0A, 0x16, 0x03,                //         UsageId(Property: Reporting State[0x0316])
    0xA1, 0x02,                      //         Collection(Logical)
    0x1A, 0x40, 0x08,                //             UsageIdMin(Reporting State: Report No Events[0x0840])
    0x2A, 0x41, 0x08,                //             UsageIdMax(Reporting State: Report All Events[0x0841])
    0x25, 0x02,                      //             LogicalMaximum(2)
    0x75, 0x02,                      //             ReportSize(2)
    0xB1, 0x00,                      //             Feature(Data, Array, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, NonVolatile, BitField)
    0xC0,                            //         EndCollection()
    0x0A, 0x19, 0x03,                //         UsageId(Property: Power State[0x0319])
    0xA1, 0x02,                      //         Collection(Logical)
    0x1A, 0x50, 0x08,                //             UsageIdMin(Power State: Undefined[0x0850])
    0x2A, 0x55, 0x08,                //             UsageIdMax(Power State: D4 Power Off[0x0855])
    0x25, 0x06,                      //             LogicalMaximum(6)
    0x75, 0x03,                      //             ReportSize(3)
    0xB1, 0x00,                      //             Feature(Data, Array, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, NonVolatile, BitField)
    0xC0,                            //         EndCollection()
    0x0A, 0x0E, 0x03,                //         UsageId(Property: Report Interval[0x030E])
    0x15, 0x00,                      //         LogicalMinimum(0)
    0x26, 0xFF, 0x0F,                //         LogicalMaximum(4,095)
    0x75, 0x0C,                      //         ReportSize(12)
    0xB1, 0x02,                      //         Feature(Data, Variable, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, NonVolatile, BitField)
    0x75, 0x07,                      //         ReportSize(7)
    0xB1, 0x03,                      //         Feature(Constant, Variable, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, NonVolatile, BitField)
    0xC0,                            //     EndCollection()
    0xC0,                            // EndCollection()
};

// Invoked when received GET HID REPORT DESCRIPTOR
// Application return pointer to descriptor
// Descriptor contents must exist long enough for transfer to complete
uint8_t const * tud_hid_descriptor_report_cb(uint8_t instance)
{
  (void) instance;
  return desc_hid_report;
}

//--------------------------------------------------------------------+
// Configuration Descriptor
//--------------------------------------------------------------------+

enum
{
  ITF_NUM_HID,
  ITF_NUM_TOTAL
};

#define CONFIG_TOTAL_LEN  (9 + 9 + 9 + 7)

#define EPNUM_HID   0x81

uint8_t const desc_configuration[] =
{
  // Configuration Descriptor
  9,                                          // bLength
  TUSB_DESC_CONFIGURATION,                    // bDescriptorType
  U16_TO_U8S_LE(CONFIG_TOTAL_LEN),           // wTotalLength
  ITF_NUM_TOTAL,                              // bNumInterfaces
  1,                                          // bConfigurationValue
  0,                                          // iConfiguration
  TUSB_DESC_CONFIG_ATT_REMOTE_WAKEUP,        // bmAttributes
  TUSB_DESC_CONFIG_POWER_MA(100),            // bMaxPower

  // Interface Descriptor
  9,                                          // bLength
  TUSB_DESC_INTERFACE,                        // bDescriptorType
  ITF_NUM_HID,                                // bInterfaceNumber
  0,                                          // bAlternateSetting
  1,                                          // bNumEndpoints
  TUSB_CLASS_HID,                             // bInterfaceClass
  0,                                          // bInterfaceSubClass
  0,                                          // bInterfaceProtocol (None)
  0,                                          // iInterface

  // HID Descriptor
  9,                                          // bLength
  HID_DESC_TYPE_HID,                          // bDescriptorType
  U16_TO_U8S_LE(0x0111),                     // bcdHID
  0,                                          // bCountryCode
  1,                                          // bNumDescriptors
  HID_DESC_TYPE_REPORT,                       // bDescriptorType
  U16_TO_U8S_LE(sizeof(desc_hid_report)),    // wDescriptorLength

  // Endpoint Descriptor
  7,                                          // bLength
  TUSB_DESC_ENDPOINT,                         // bDescriptorType
  EPNUM_HID,                                  // bEndpointAddress
  TUSB_XFER_INTERRUPT,                        // bmAttributes
  U16_TO_U8S_LE(CFG_TUD_HID_EP_BUFSIZE),     // wMaxPacketSize
  5                                           // bInterval
};

#if TUD_OPT_HIGH_SPEED
// Per USB specs: high speed capable device must report device_qualifier and other_speed_configuration

// other speed configuration
uint8_t desc_other_speed_config[CONFIG_TOTAL_LEN];

// device qualifier is mostly similar to device descriptor except max packet size for endpoint0 and number of possible configurations
tusb_desc_device_qualifier_t const desc_device_qualifier =
{
  .bLength            = sizeof(tusb_desc_device_qualifier_t),
  .bDescriptorType    = TUSB_DESC_DEVICE_QUALIFIER,
  .bcdUSB             = USB_BCD,

  .bDeviceClass       = 0x00,
  .bDeviceSubClass    = 0x00,
  .bDeviceProtocol    = 0x00,

  .bMaxPacketSize0    = CFG_TUD_ENDPOINT0_SIZE,
  .bNumConfigurations = 0x01,
  .bReserved          = 0x00
};

// Invoked when received GET DEVICE QUALIFIER DESCRIPTOR request
// Application return pointer to descriptor, whose contents must exist long enough for transfer to complete.
// device_qualifier descriptor describes information about a high-speed capable device that would
// change if the device were operating at the other speed. If not highspeed capable stall this request.
uint8_t const* tud_descriptor_device_qualifier_cb(void)
{
  return (uint8_t const*) &desc_device_qualifier;
}

// Invoked when received GET OTHER SEED CONFIGURATION DESCRIPTOR request
// Application return pointer to descriptor, whose contents must exist long enough for transfer to complete
// Configuration descriptor in the other speed e.g if high speed then this is for full speed and vice versa
uint8_t const* tud_descriptor_other_speed_configuration_cb(uint8_t index)
{
  (void) index; // for multiple configurations

  // other speed config is basically configuration with type = OHER_SPEED_CONFIG
  memcpy(desc_other_speed_config, desc_configuration, CONFIG_TOTAL_LEN);
  desc_other_speed_config[1] = TUSB_DESC_OTHER_SPEED_CONFIG;

  return desc_other_speed_config;
}
#endif // highspeed

// Invoked when received GET CONFIGURATION DESCRIPTOR
uint8_t const * tud_descriptor_configuration_cb(uint8_t index)
{
  (void) index; // for multiple configurations

  // Use the same configuration for both high and full speed mode.
  // Unsure of the consequences
  return desc_configuration;
}

//--------------------------------------------------------------------+
// String Descriptors
//--------------------------------------------------------------------+

// // array of pointer to string descriptors
// char const* string_desc_arr [] =
// {
//   (const char[]) { 0x09, 0x04 }, // 0: is supported language is English (0x0409)
//   "Raspberry Pi",                // 1: Manufacturer
//   "RP2040 ALS HID Sensor",       // 2: Product
//   NULL,                          // 3: Serials, should use chip ID
// };
//
// static uint16_t _desc_str[32];
//
// // Invoked when received GET STRING DESCRIPTOR request
// // Application return pointer to descriptor, whose contents must exist long enough for transfer to complete
// uint16_t const* tud_descriptor_string_cb(uint8_t index, uint16_t langid)
// {
//   (void) langid;
//
//   uint8_t chr_count;
//
//   if ( index == 0)
//   {
//     memcpy(&_desc_str[1], string_desc_arr[0], 2);
//     chr_count = 1;
//   }
//   else if ( index == 3 )
//   {
//     // Get unique serial number from RP2040 chip ID
//     pico_unique_board_id_t board_id;
//     pico_get_unique_board_id(&board_id);
//
//     char serial_str[17];  // 8 bytes = 16 hex chars + null terminator
//     for(int i = 0; i < 8; i++) {
//       sprintf(&serial_str[i * 2], "%02X", board_id.id[i]);
//     }
//     serial_str[16] = 0;
//
//     chr_count = 16;
//     for(int i = 0; i < chr_count; i++) {
//       _desc_str[1 + i] = serial_str[i];
//     }
//   }
//   else
//   {
//
//     if ( !(index < sizeof(string_desc_arr)/sizeof(string_desc_arr[0])) ) return NULL;
//
//     const char* str = string_desc_arr[index];
//
//     // Cap at max char
//     chr_count = strlen(str);
//     if ( chr_count > 31 ) chr_count = 31;
//
//     // Convert ASCII string into UTF-16
//     for(uint8_t i=0; i<chr_count; i++)
//     {
//       _desc_str[1+i] = str[i];
//     }
//   }
//
//   // first byte is length (including header), second byte is string type
//   _desc_str[0] = (TUSB_DESC_STRING << 8 ) | (2*chr_count + 2);
//
//   return _desc_str;
// }