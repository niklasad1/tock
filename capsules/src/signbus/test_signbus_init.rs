// Capsule to test signbus intialization functions in tock
// By: Justin Hsieh


pub struct SignbusIntialization<'a> {

	// USE AS NEEDED
	app_layer: 			&'a app_layer::SignbusAppLayer<'a>,
	protocol_layer: 	&'a protocol_layer::SignbusProtocolLayer<'a>,
	io_layer: 			&'a io_layer::SignbusIOLayer<'a>,
	port_layer: 		&'a port_layer::SignbusPortLayer<'a>,

	client: Cell<Option<&'static port_layer::SignbusPortLayer<'static>>>,
	client: Cell<Option<&'static app_layer::SignbusAppLayer<'static>>>,
}

impl<'a> SignbusIntialization <'a> {
	pub fn new(app_layer: 			&'a app_layer::SignbusAppLayer,
				protocol_layer: 	&'a protocol_layer::SignbusProtocolLayer,
				io_layer: 			&'a io_layer::SignbusIOLayer,
				port_layer: 		&'a port_layer::SignbusPortLayer,
	
	) -> SignbusIntialization <'a> {
		
		SignbusIntialization {
			app_layer:  		app_layer,
			protocol_layer:  	protocol_layer,
			io_layer:  			io_layer,
			port_layer:  		port_layer,
		}
	}


}



	




