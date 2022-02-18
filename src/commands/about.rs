use serenity::{
    client::Context, model::interactions::application_command::ApplicationCommandInteraction,
};
use std::borrow::{Borrow, BorrowMut};

use crate::utils::send_response_complex;
use crate::{config::Command, error::ExecutionError, utils::send_response};

pub async fn execute(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    command_config: &Command,
) -> Result<(), ExecutionError> {
    send_response(
        &ctx,
        &command,
        command_config,
        "About Kowalski",
        "[Kowalski](https://github.com/simonpannek/kowalski-rs) is a small discord bot \
        including some utility commands, reaction-roles and a level up system using reactions.

        **Author:**
        The bot is currently being developed by me, [Simon Pannek](https://pannek.dev) :)
        If there is anything wrong, feel free to reach out to me on Discord \
        ([simon#9876](https://discordapp.com/users/158280426551640064)).",
    )
    .await
}
