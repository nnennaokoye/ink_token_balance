#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod token_contract {
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct TokenContract {
        /// The owner (admin) allowed to mint new tokens.
        owner: AccountId,
        /// Mapping from account to token balance.
        balances: Mapping<AccountId, u128>,
        /// Total supply of all minted tokens.
        total_supply: u128,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Caller attempted an owner-only action.
        NotOwner,
        /// Sender tried to transfer more than their balance.
        InsufficientBalance,
        /// Arithmetic overflow/underflow detected.
        ArithmeticError,
        /// Provided amount must be greater than zero.
        ZeroAmount,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    /// Emitted when tokens are minted to an account.
    #[ink(event)]
    pub struct Minted {
        #[ink(topic)]
        to: AccountId,
        amount: u128,
    }

    /// Emitted when tokens are transferred between accounts.
    #[ink(event)]
    pub struct Transferred {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        amount: u128,
    }

    impl Default for TokenContract {
        fn default() -> Self {
            Self::new()
        }
    }

    impl TokenContract {
        /// Initializes the token contract setting the deployer as the owner.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                owner: Self::env().caller(),
                balances: Mapping::default(),
                total_supply: 0,
            }
        }

        /// Returns the token balance of the given account.
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> u128 {
            self.balances.get(owner).unwrap_or(0)
        }

        /// Returns the total supply of minted tokens.
        #[ink(message)]
        pub fn total_supply(&self) -> u128 {
            self.total_supply
        }

        /// Mints `amount` tokens to `to`. Only the contract owner may call this.
        #[ink(message)]
        pub fn mint(&mut self, to: AccountId, amount: u128) -> Result<()> {
            if Self::env().caller() != self.owner {
                return Err(Error::NotOwner);
            }
            if amount == 0 {
                return Err(Error::ZeroAmount);
            }

            let current = self.balances.get(to).unwrap_or(0);
            let new_balance = current.checked_add(amount).ok_or(Error::ArithmeticError)?;
            self.balances.insert(to, &new_balance);

            // Increase total supply safely
            self.total_supply = self
                .total_supply
                .checked_add(amount)
                .ok_or(Error::ArithmeticError)?;

            self.env().emit_event(Minted { to, amount });
            Ok(())
        }

        /// Transfers `amount` tokens from the caller to `to`.
        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, amount: u128) -> Result<()> {
            if amount == 0 {
                return Err(Error::ZeroAmount);
            }

            let from = self.env().caller();
            let from_balance = self.balances.get(from).unwrap_or(0);
            if from_balance < amount {
                return Err(Error::InsufficientBalance);
            }

            let new_from = from_balance
                .checked_sub(amount)
                .ok_or(Error::ArithmeticError)?;
            let to_balance = self.balances.get(to).unwrap_or(0);
            let new_to = to_balance.checked_add(amount).ok_or(Error::ArithmeticError)?;

            self.balances.insert(from, &new_from);
            self.balances.insert(to, &new_to);

            self.env().emit_event(Transferred { from, to, amount });
            Ok(())
        }
    }
}