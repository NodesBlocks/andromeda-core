use cosmwasm_std::{DepsMut, Env, MessageInfo, StdError, StdResult, Storage};
use cw_storage_plus::Map;

use crate::{
    modules::{
        common::{is_unique, require},
        hooks::{HookResponse, MessageHooks},
        {Module, ModuleDefinition},
    },
    token::ExecuteMsg,
};

pub const BLACKLIST: Map<String, bool> = Map::new("blacklist");

pub struct Blacklist {
    pub moderators: Vec<String>,
}

impl Blacklist {
    pub fn is_moderator(&self, addr: &String) -> bool {
        self.moderators.contains(addr)
    }
    pub fn blacklist_addr(&self, storage: &mut dyn Storage, addr: &String) -> StdResult<()> {
        BLACKLIST.save(storage, addr.clone(), &true)
    }
    pub fn remove_blacklist(&self, storage: &mut dyn Storage, addr: &String) -> StdResult<()> {
        BLACKLIST.save(storage, addr.clone(), &false)
    }
    pub fn is_blacklisted(&self, storage: &dyn Storage, addr: &String) -> StdResult<bool> {
        match BLACKLIST.load(storage, addr.clone()) {
            Ok(whitelisted) => Ok(whitelisted),
            Err(e) => match e {
                cosmwasm_std::StdError::NotFound { .. } => Ok(false),
                _ => Err(e),
            },
        }
    }
}

impl Module for Blacklist {
    fn validate(&self, all_modules: Vec<ModuleDefinition>) -> StdResult<bool> {
        require(
            is_unique(self, &all_modules),
            StdError::generic_err("Blacklist module must be unique"),
        )?;

        let contains_whitelist = all_modules.iter().any(|m| match m {
            &ModuleDefinition::Whitelist { .. } => true,
            _ => false,
        });
        require(
            !contains_whitelist,
            StdError::generic_err("Cannot have both a blacklist and a whitelist"),
        )?;

        Ok(true)
    }
    fn as_definition(&self) -> ModuleDefinition {
        ModuleDefinition::Blacklist {
            moderators: self.moderators.to_vec(),
        }
    }
}

impl MessageHooks for Blacklist {
    fn on_execute(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        _env: Env,
        _msg: ExecuteMsg,
    ) -> StdResult<HookResponse> {
        require(
            !self.is_blacklisted(deps.storage, &info.sender.to_string())?,
            StdError::generic_err("Address is blacklisted"),
        )?;

        Ok(HookResponse::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_validate() {
        let bl = Blacklist { moderators: vec![] };
        let mut modules = vec![
            bl.as_definition().clone(),
            ModuleDefinition::Taxable {
                tax: 2,
                receivers: vec![],
            },
        ];

        assert_eq!(bl.validate(modules.to_vec()), Ok(true));

        modules.push(bl.as_definition().clone());

        assert_eq!(
            bl.validate(modules.to_vec()),
            Err(StdError::generic_err("Blacklist module must be unique"))
        );

        let modules = vec![
            bl.as_definition().clone(),
            ModuleDefinition::Taxable {
                tax: 2,
                receivers: vec![],
            },
            ModuleDefinition::Whitelist { moderators: vec![] },
        ];

        assert_eq!(
            bl.validate(modules.to_vec()),
            Err(StdError::generic_err(
                "Cannot have both a blacklist and a whitelist"
            ))
        );
    }

    #[test]
    fn test_on_execute() {
        let sender = String::from("sender");
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("sender", &[]);
        let wl = Blacklist { moderators: vec![] };
        let msg = ExecuteMsg::Revoke {
            spender: String::default(),
            token_id: String::default(),
        };

        let resp = wl
            .on_execute(&deps.as_mut(), info.clone(), env.clone(), msg.clone())
            .unwrap();

        assert_eq!(resp, HookResponse::default());

        wl.blacklist_addr(&mut deps.storage, &sender.clone())
            .unwrap();

        let resp = wl
            .on_execute(&deps.as_mut(), info.clone(), env.clone(), msg.clone())
            .unwrap_err();

        assert_eq!(resp, StdError::generic_err("Address is blacklisted"));
    }
}