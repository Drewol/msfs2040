use usbd_hid::descriptor::generator_prelude::*;

#[gen_hid_descriptor(
    (collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = GAMEPAD) = {
        (collection = PHYSICAL, usage = GAMEPAD) = {
                (usage_page = GENERIC_DESKTOP,) = {
                    ( usage = 0x01,) = {
                        #[item_settings data,variable,absolute]
                        output_buffer = input;
                    };   
                };
                (usage_page = GENERIC_DESKTOP,) = {
                    ( usage = 0x02,) = {
                        #[item_settings data,variable,absolute]
                        input_buffer = output;
                    };   
                };
        };
        
    }
    )
    ]
pub struct DeviceData {
    pub output_buffer: [u8; 32],
    pub input_buffer: [u8; 32],
}
