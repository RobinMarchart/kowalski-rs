use serenity::{prelude::Context, json::Value};

use crate::error::KowalskiError;

pub mod scheduled_events;

pub async fn unknown(
    ctx:&Context,
    name:String,
    raw:Value
)-> Result<(),KowalskiError>{
    Ok(())
}
