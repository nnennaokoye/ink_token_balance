#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod simple_token {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct SimpleToken {
        balances: Mapping<AccountId, u128>,
        allowances: Mapping<(AccountId, AccountId), u128>,
        owner: AccountId,
        total_supply: u128,
        paused: bool,
        blacklist: Mapping<AccountId, bool>,
    }

    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {
        InsufficientBalance,
        NotOwner,
        SelfTransfer,
        Overflow,
        InsufficientAllowance,
        Paused,
        Blacklisted,
        BatchLengthMismatch,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: AccountId,
        value: u128,
    }

    #[ink(event)]
    pub struct Mint {
        #[ink(topic)]
        to: AccountId,
        value: u128,
    }

    #[ink(event)]
    pub struct Burn {
        #[ink(topic)]
        from: AccountId,
        value: u128,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        value: u128,
    }

    #[ink(event)]
    pub struct Paused {
        paused: bool,
    }

    #[ink(event)]
    pub struct BlacklistUpdated {
        #[ink(topic)]
        account: AccountId,
        blacklisted: bool,
    }

    impl SimpleToken {
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            Self {
                balances: Mapping::default(),
                allowances: Mapping::default(),
                owner: caller,
                total_supply: 0,
                paused: false,
                blacklist: Mapping::default(),
            }
        }

        #[ink(message)]
        pub fn mint(&mut self, to: AccountId, amount: u128) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotOwner);
            }

            if self.is_blacklisted(to) {
                return Err(Error::Blacklisted);
            }

            let current_balance = self.balances.get(to).unwrap_or(0);
            let new_balance = current_balance.checked_add(amount).ok_or(Error::Overflow)?;
            self.balances.insert(to, &new_balance);
            
            self.total_supply = self.total_supply.checked_add(amount).ok_or(Error::Overflow)?;

            self.env().emit_event(Mint { to, value: amount });

            Ok(())
        }

        #[ink(message)]
        pub fn burn(&mut self, amount: u128) -> Result<(), Error> {
            let caller = self.env().caller();
            let balance = self.balance_of(caller);

            if balance < amount {
                return Err(Error::InsufficientBalance);
            }

            let new_balance = balance.checked_sub(amount).ok_or(Error::Overflow)?;
            self.balances.insert(caller, &new_balance);
            
            self.total_supply = self.total_supply.checked_sub(amount).ok_or(Error::Overflow)?;

            self.env().emit_event(Burn { from: caller, value: amount });

            Ok(())
        }

        #[ink(message)]
        pub fn balance_of(&self, account: AccountId) -> u128 {
            self.balances.get(account).unwrap_or(0)
        }

        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, amount: u128) -> Result<(), Error> {
            let caller = self.env().caller();
            self.transfer_from_to(caller, to, amount)
        }

        #[ink(message)]
        pub fn approve(&mut self, spender: AccountId, amount: u128) -> Result<(), Error> {
            let caller = self.env().caller();
            
            self.allowances.insert((caller, spender), &amount);

            self.env().emit_event(Approval {
                owner: caller,
                spender,
                value: amount,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn allowance(&self, owner: AccountId, spender: AccountId) -> u128 {
            self.allowances.get((owner, spender)).unwrap_or(0)
        }

        #[ink(message)]
        pub fn transfer_from(&mut self, from: AccountId, to: AccountId, amount: u128) -> Result<(), Error> {
            let caller = self.env().caller();
            let allowance = self.allowance(from, caller);

            if allowance < amount {
                return Err(Error::InsufficientAllowance);
            }

            self.transfer_from_to(from, to, amount)?;

            let new_allowance = allowance.checked_sub(amount).ok_or(Error::Overflow)?;
            self.allowances.insert((from, caller), &new_allowance);

            Ok(())
        }

        #[ink(message)]
        pub fn batch_transfer(&mut self, recipients: Vec<AccountId>, amounts: Vec<u128>) -> Result<(), Error> {
            if recipients.len() != amounts.len() {
                return Err(Error::BatchLengthMismatch);
            }

            let caller = self.env().caller();

            for (to, amount) in recipients.iter().zip(amounts.iter()) {
                self.transfer_from_to(caller, *to, *amount)?;
            }

            Ok(())
        }

        #[ink(message)]
        pub fn pause(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotOwner);
            }

            self.paused = true;
            self.env().emit_event(Paused { paused: true });

            Ok(())
        }

        #[ink(message)]
        pub fn unpause(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotOwner);
            }

            self.paused = false;
            self.env().emit_event(Paused { paused: false });

            Ok(())
        }

        #[ink(message)]
        pub fn set_blacklist(&mut self, account: AccountId, blacklisted: bool) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotOwner);
            }

            self.blacklist.insert(account, &blacklisted);
            self.env().emit_event(BlacklistUpdated { account, blacklisted });

            Ok(())
        }

        #[ink(message)]
        pub fn is_blacklisted(&self, account: AccountId) -> bool {
            self.blacklist.get(account).unwrap_or(false)
        }

        #[ink(message)]
        pub fn is_paused(&self) -> bool {
            self.paused
        }

        #[ink(message)]
        pub fn total_supply(&self) -> u128 {
            self.total_supply
        }

        #[ink(message)]
        pub fn owner(&self) -> AccountId {
            self.owner
        }

        fn transfer_from_to(&mut self, from: AccountId, to: AccountId, amount: u128) -> Result<(), Error> {
            if self.paused {
                return Err(Error::Paused);
            }

            if from == to {
                return Err(Error::SelfTransfer);
            }

            if self.is_blacklisted(from) || self.is_blacklisted(to) {
                return Err(Error::Blacklisted);
            }

            let from_balance = self.balance_of(from);
            if from_balance < amount {
                return Err(Error::InsufficientBalance);
            }
            
            let new_from_balance = from_balance.checked_sub(amount).ok_or(Error::Overflow)?;
            self.balances.insert(from, &new_from_balance);

            let to_balance = self.balance_of(to);
            let new_to_balance = to_balance.checked_add(amount).ok_or(Error::Overflow)?;
            self.balances.insert(to, &new_to_balance);

            self.env().emit_event(Transfer {
                from: Some(from),
                to,
                value: amount,
            });

            Ok(())
        }
    }
}
