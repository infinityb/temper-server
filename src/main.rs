extern crate usb;
extern crate dbus;
extern crate temper;

use std::sync::Mutex;
use temper::{Temper, TemperReadErr};
use dbus::{Connection, BusType, NameFlag, ConnectionItem, Message, MessageItem};
use dbus::obj::{ObjectPath, Argument, Method, Interface};

static DBUS_ERROR_FAILED: &'static str = "org.freedesktop.DBus.Failed";
static BUS_NAME: &'static str = "org.yasashiisyndicate.TemperServer";
static THERMOMETER_IFACE: &'static str = "org.yasashiisyndicate.TemperServer";

fn temper_to_dbus(res: Result<f64, TemperReadErr>) -> dbus::obj::MethodResult {
    match res {
        Ok(temperature) => Ok(vec!(MessageItem::Double(temperature))),
        Err(error) => Err((DBUS_ERROR_FAILED, format!("{:?}", error))),
    }
}

fn main() {
    let usbctx = usb::Context::new();
    let dev = usbctx.find_by_vid_pid(0x0C45, 0x7401).expect("Device not found");
    let temper = Mutex::new(Temper::new(dev.open().ok().expect("Device open failed")));

    let c = Connection::get_private(BusType::Session).unwrap();
    c.register_name(BUS_NAME, NameFlag::ReplaceExisting as u32).unwrap();
    let mut o = ObjectPath::new(&c, "/Thermometer", true);
    o.insert_interface(THERMOMETER_IFACE, Interface::new(
        vec!(Method::new("GetTemperature",
            vec!(), // No input arguments
            vec!(Argument::new("reply", "d")),
            Box::new(|_msg| {
                match temper.lock() {
                    Ok(mut temper) => temper_to_dbus(temper.get_temperature()),
                    Err(_) => Err((DBUS_ERROR_FAILED, "poisoned".to_string())),
                }
            }),
        )),
        vec!(), vec!() // No properties or signals
    ));

    o.set_registered(true).unwrap();

    for n in c.iter(1000) {
        match n {
            ConnectionItem::MethodCall(mut m) => {
                println!("MethodCall: {:?}", m);
                if o.handle_message(&mut m).is_none() {
                    c.send(Message::new_error(&m, DBUS_ERROR_FAILED, "Object path not found").unwrap()).unwrap();
                };
            },
            ConnectionItem::Signal(m) => {
                println!("Signal: {:?}", m);
            },
            ConnectionItem::Nothing => (),
        }
    }
}
