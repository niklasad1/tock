// Capsule to test signbus intialization functions in tock
// By: Justin Hsieh

use core::cell::Cell;
use kernel::common::take_cell::TakeCell;
use signbus;
use signbus::{io_layer, support, app_layer, port_layer, protocol_layer};

pub static mut BUFFER0: [u8; 255] = [0; 255];
pub static mut BUFFER1: [u8; 255] = [0; 255];

pub enum ModuleAddress {
    Controller = 0x20,
    Storage = 0x21,
    Radio = 0x22,
}

#[derive(Clone,Copy,PartialEq)]
pub enum DelayState {
    Idle,
    RequestIsolation,
}


pub struct SignbusInitialization<'a> {
    // USE AS NEEDED
    app_layer: &'a app_layer::SignbusAppLayer<'a>,
    protocol_layer: &'a protocol_layer::SignbusProtocolLayer<'a>,
    io_layer: &'a io_layer::SignbusIOLayer<'a>,
    port_layer: &'a port_layer::PortLayer,

    delay_state: Cell<DelayState>,
    send_buf: TakeCell<'static, [u8]>,

    // INCOMING MESSAGE STORAGE
    source_address: Cell<u8>,
    frame_type: Cell<support::SignbusFrameType>,
    api_type: Cell<support::SignbusApiType>,
    message_type: Cell<support::InitMessageType>,
    length: Cell<usize>,
    recv_buf: TakeCell<'static, [u8]>,
}

impl<'a> SignbusInitialization<'a> {
    pub fn new(app_layer: &'a app_layer::SignbusAppLayer,
               protocol_layer: &'a protocol_layer::SignbusProtocolLayer,
               io_layer: &'a io_layer::SignbusIOLayer,
               port_layer: &'a port_layer::PortLayer,
               send_buf: &'static mut [u8],
               recv_buf: &'static mut [u8])
               -> SignbusInitialization<'a> {

        SignbusInitialization {
            app_layer: app_layer,
            protocol_layer: protocol_layer,
            io_layer: io_layer,
            port_layer: port_layer,

            source_address: Cell::new(0),
            frame_type: Cell::new(support::SignbusFrameType::NotificationFrame),
            api_type: Cell::new(support::SignbusApiType::InitializationApiType),
            message_type: Cell::new(support::InitMessageType::Declare),
            length: Cell::new(0),
            recv_buf: TakeCell::new(recv_buf),

            delay_state: Cell::new(DelayState::Idle),
            send_buf: TakeCell::new(send_buf),
        }
    }

    pub fn signpost_initialization_declare_controller(&self) {
        debug!("Declare controller...");

        self.send_buf.take().map(|buf| {
            buf[0] = 0x32;

            self.app_layer.signbus_app_send(ModuleAddress::Controller as u8,
                                            support::SignbusFrameType::CommandFrame,
                                            support::SignbusApiType::InitializationApiType,
                                            support::InitMessageType::Declare as u8,
                                            1,
                                            buf);
        });
    }

    pub fn signpost_initialization_request_isolation(&self) {
        debug!("Request I2C isolation");
        // intialize mod out/in gpio
        self.port_layer.mod_out_set();
        self.port_layer.debug_led_off();
        self.port_layer.mod_in_enable_interrupt();

        // pull mod out low to signal controller
        // wait on controller interrupt on mod_in
        self.port_layer.mod_out_clear();
        self.port_layer.debug_led_on();
    }


    pub fn signpost_initialization_module_init(&self, i2c_address: u8) {
        debug!("Start Initialization");
        // intialize lower layers
        self.io_layer.signbus_io_init(i2c_address);

        // listen for messages
        self.recv_buf.take().map(|buf| { self.app_layer.signbus_app_recv(buf); });

        // communicate with controller and request 1:1 talk (isolation)
        self.signpost_initialization_request_isolation();
    }
}

impl<'a> port_layer::PortLayerClient2 for SignbusInitialization<'a> {
    // Called when the mod_in GPIO goes low.
    fn mod_in_interrupt(&self) {
        debug!("Interrupt!");
        self.delay_state.set(DelayState::RequestIsolation);
        self.port_layer.delay_ms(50);
    }

    // Called when a delay_ms has completed.
    fn delay_complete(&self) {
        debug!("Fired!");
        match self.delay_state.get() {

            DelayState::Idle => {}

            DelayState::RequestIsolation => {
                if self.port_layer.mod_in_read() != 0 {
                    debug!("Spurrious interrupt");
                    return;
                }
                self.signpost_initialization_declare_controller();
            }
        }
    }
}


impl<'a> app_layer::AppLayerClient for SignbusInitialization<'a> {
    // Called when a new packet is received over I2C.
    fn packet_received(&self, data: &'static mut [u8], length: usize, error: support::Error) {

        match error {
            support::Error::AddressNak => debug!("Error: AddressNak"),
            support::Error::DataNak => debug!("Error: DataNak"),
            support::Error::ArbitrationLost => debug!("Error: ArbitrationNak"),
            support::Error::CommandComplete => debug!("Command Complete!"),
        };

        // signpost_initialization_declared_callback
        if length < 0 {
            // check incoming_api_type and incoming_message_type
            // self.frame_type.set(data[8] as support::SignbusFrameType);
            // self.api_type.set(data[9] as support::SignbusApiType);
            // self.message_type.set(data[10] as InitMessageType);

            if data[1] == support::SignbusApiType::InitializationApiType as u8 &&
               data[2] == support::InitMessageType::Declare as u8 {
                debug!("Correct response for declaration.");
            } else {
                debug!("Incorrect response for declaration.");
            }

        } else {
            debug!("Error: Length = 0");
        }
        self.send_buf.replace(data);
    }
    // Called when an I2C master write command is complete.
    fn packet_sent(&self, data: &'static mut [u8], error: support::Error) {
        self.send_buf.replace(data);
    }


    // Called when an I2C slave read has completed.
    fn packet_read_from_slave(&self) {}
}
