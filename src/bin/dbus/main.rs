use std::str::FromStr;

use async_recursion::async_recursion;
use zbus::fdo::{DBusProxy};
use zbus::names::OwnedBusName;
use zbus::xml::{Node, Interface};
use zbus::zvariant::ObjectPath;
use zbus::{Connection, Result};

#[async_recursion]
async fn print_all_interfaces(connection: &Connection, service: &OwnedBusName, path: ObjectPath<'async_recursion>, indent: usize) -> std::result::Result<(), zbus::Error>{

    println!(
        "{:indent$}{} ", "",
        path.as_str(),
            indent=indent);
    let introspectable_proxy = zbus::fdo::IntrospectableProxy::builder(&connection)
        .destination(service)?
        .path(path.clone())?
        .build()
        .await?;
    let introspect_xml = introspectable_proxy.introspect().await?;
    let introspect = Node::from_str(&introspect_xml)?;
    println!(
        "{:indent$}Interfaces: ", "",
            indent=indent+4);
    for interface in introspect.interfaces() {
        println!(
            "{:indent$}{} ", "",
            interface.name(),
                indent=indent+8);
        println!(
            "{:indent$}Methods: ", "", indent=indent+12);
        for method in interface.methods() {
            println!(
                "{:indent$}{} ", "",
                method.name(),
                    indent=indent+16);
        }
        println!(
            "{:indent$}Signals: ", "", indent=indent+12);
        for signal in interface.signals() {
            println!(
                "{:indent$}{} ", "",
                signal.name(),
                    indent=indent+16);
        }
        println!(
            "{:indent$}Properties: ", "", indent=indent+12);
        for property in interface.properties() {
            println!(
                "{:indent$}{} ", "",
                property.name(),
                    indent=indent+16);
        }
        println!(
            "{:indent$}Annotations: ", "", indent=indent+12);
        for annotation in interface.annotations() {
            println!(
                "{:indent$}{} ", "",
                annotation.name(),
                    indent=indent+16);
        }
    }
    for node in introspect.nodes() {
        let node_name = node.name().unwrap();
        
        let path_name = if path.as_str().ends_with('/'){
            path.as_str().to_string() + node_name
        }
        else {
            path.as_str().to_string() + "/" + node_name
        };
        let sub_path = ObjectPath::try_from(path_name)?;

        print_all_interfaces(connection, service, sub_path, indent).await?;
    }
    Ok(())

}
#[tokio::main]
async fn main() -> Result<()> {
    let connection = Connection::session().await?;

    let dbusproxy = DBusProxy::new(&connection).await?;
    let reply = dbusproxy.list_names().await?;
    let _service = reply.first().unwrap();
    for service in reply {
        if service.as_str().contains(':') {
            continue;
        }
        println!("Service: {}", service.as_str());
        let path_name = "/".to_string();
        let path = ObjectPath::try_from(path_name)?;
        print_all_interfaces(&connection, &service, path, 4).await?;
    }

    Ok(())
}
