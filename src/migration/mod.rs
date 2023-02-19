use std::collections::HashMap;

use serenity::{Client, async_trait};

use crate::{database::client::Database, error::KowalskiError,migrations};

use self::given_roles::GivenRoles;

mod given_roles;
#[async_trait]
pub trait Migration{
    async fn migrate(&self,db:&Database,client:&Client)->Result<(),KowalskiError>;
}

pub fn migrations()->HashMap<&'static str,Box<dyn Migration>>{
    migrations!(
        (GivenRoles,"given_roles")
    )
}

