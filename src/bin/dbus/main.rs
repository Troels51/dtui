use std::str::FromStr;

use zbus::blocking::fdo::IntrospectableProxy;
use zbus::{Connection, Result, dbus_proxy};
use zbus::fdo::DBusProxy;
use zbus::xml::Node;
use zbus::zvariant::ObjectPath;

#[tokio::main]
async fn main() -> Result<()> {
    let connection = Connection::session().await?;

    // `dbus_proxy` macro creates `MyGreaterProxy` based on `Notifications` trait.
    let dbusproxy = DBusProxy::new(&connection).await?;
    let reply = dbusproxy.list_names().await?;
    let service = reply.first().unwrap();
    for service in reply {
        if service.as_str().contains(":") {
            continue;
        }
        println!("Service: {}", service.as_str());
        let path_name = "/".to_string() + &(service.as_str().replace(".", "/"));
        println!("Path name {}", path_name);
        let path = ObjectPath::try_from(path_name)?;
        let objectManagerProxy = zbus::fdo::ObjectManagerProxy::builder(&connection)
            .destination(&service)?
            .path(&path)?
            .build().await?;
        let objects = objectManagerProxy.get_managed_objects().await;
        dbg!(objects);
        let introspectableProxy = zbus::fdo::IntrospectableProxy::builder(&connection)
            .destination(&service)?
            .path(&path)?
            .build().await?;

        let introspect_xml = introspectableProxy.introspect().await?;
        let introspect = Node::from_str(&introspect_xml);
        print!("{} \n", introspect.unwrap().interfaces().into_iter().map(|interface|{
            interface.name()
            }).collect::<Vec<&str>>().join("\n")
        );

    }

    //dbg!(reply);

    Ok(())
}