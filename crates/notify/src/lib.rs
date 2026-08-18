use rustbus::connection::Timeout;
use rustbus::connection::ll_conn::force_finish_on_error;
use rustbus::message_builder::{HeaderFlags, MessageBuilder};
use rustbus::wire::marshal::traits::Variant;
use rustbus::RpcConn;
use std::collections::HashMap;

/// Sends a desktop notification via the session bus's `org.freedesktop.Notifications.Notify`. 
pub fn notify(summary: &str, body: &str) {
    if let Err(err) = try_notify(summary, body) {
        eprintln!("notification failed: {err}");
    }
}

fn try_notify(summary: &str, body: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = RpcConn::session_conn(Timeout::Infinite)?;

    let mut call = MessageBuilder::new()
        .call("Notify")
        .with_interface("org.freedesktop.Notifications")
        .on("/org/freedesktop/Notifications")
        .at("org.freedesktop.Notifications")
        .build();

    HeaderFlags::NoReplyExpected.set(&mut call.flags);
    
    call.body.push_param("ricture")?; // app_name
    call.body.push_param(0u32)?; // replaces_id
    call.body.push_param("")?; // app_icon
    call.body.push_param(summary)?;
    call.body.push_param(body)?;
    call.body.push_param(Vec::<String>::new())?; 
    call.body
        .push_param(HashMap::<String, Variant<bool>>::new())?;
    call.body.push_param(-1i32)?; 

    conn.send_message(&mut call)?
        .write_all()
        .map_err(force_finish_on_error)?;
    Ok(())
}
