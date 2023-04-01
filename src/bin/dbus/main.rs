use std::str::FromStr;

use zbus::fdo::DBusProxy;
use zbus::xml::Node;
use zbus::zvariant::ObjectPath;
use zbus::{Connection, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let connection = Connection::session().await?;

    // `dbus_proxy` macro creates `MyGreaterProxy` based on `Notifications` trait.
    let dbusproxy = DBusProxy::new(&connection).await?;
    let reply = dbusproxy.list_names().await?;
    let _service = reply.first().unwrap();
    for service in reply {
        if service.as_str().contains(':') {
            continue;
        }
        println!("Service: {}", service.as_str());
        let path_name = "/".to_string() + &(service.as_str().replace('.', "/"));
        println!("Path name {}", path_name);
        let path = ObjectPath::try_from(path_name)?;
        let introspectable_proxy = zbus::fdo::IntrospectableProxy::builder(&connection)
            .destination(&service)?
            .path(&path)?
            .build()
            .await?;

        let introspect_xml = introspectable_proxy.introspect().await?;
        let introspect = Node::from_str(&introspect_xml);
        println!(
            "{} ",
            introspect
                .unwrap()
                .interfaces()
                .into_iter()
                .map(|interface| { interface.name() })
                .collect::<Vec<&str>>()
                .join("\n")
        );
    }

    Ok(())
}
