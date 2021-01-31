use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    weights::{
        GetDispatchInfo, DispatchClass, WeighData,
        Weight, ClassifyDispatch, PaysFee, Pays,
    },
    dispatch::{DispatchError, DispatchResult},
    traits::{
        Currency, Get, ExistenceRequirement,
        OriginTrait, IsType, Filter,
    },
    Parameter,
};

use pallet_utils::{StorefrontId, vec_remove_on};

use super::*;

impl<T: Trait> Order<T> {

    pub fn new(
        id: OrderId,
        created_by: T::AccountId,
        storefront_id: StorefrontId,
        product_id: ProductId,
        order_total: u32,
        seller: T::AccountId,
        buyer_escrow: u32,
        seller_escrow: u32,
        content: Content
    ) -> Self {
        Order {
            id,
            created: WhoAndWhen::<T>::new(created_by.clone()),
            updated: None,
            owner: created_by,
            order_state: OrderState::New,
            storefront_id: storefront_id,
            product_id: product_id,
            order_total: order_total,
            seller: seller,
            buyer_escrow: buyer_escrow,
            seller_escrow: seller_escrow,
            content,
        }
    }


    pub fn is_owner(&self, account: &T::AccountId) -> bool {
       self.owner == *account
   }







} // end trait order




impl Default for OrderUpdate {
    fn default() -> Self {
        OrderUpdate {
            content: None,
            order_state: OrderState::Pending
        }
    }
}


impl<T: Trait> Module<T> {

   /// Get `Order` by id from the storage or return `OrderNotFound` error.
   pub fn require_order(order_id: OrderId) -> Result<Order<T>, DispatchError> {
    Ok(Self::order_by_id(order_id).ok_or(Error::<T>::OrderNotFound)?)
}

// pub fn u32_to_balance_option(input: u32) -> Option<BalanceOf<T>> {
//   input.try_into().ok()
// }

    // Transfer tokens amount/entire free balance (if amount is `None`) from key account to owner
    // pub fn fund_buyer_escrow(
    //     buyer_account: &T::AccountId,
    //     treasur: &T::AccountId,
    //     amount: BalanceOf<T>
    // ) -> DispatchResult {
    //     T::Currency::transfer(
    //         buyer_account,
    //         treasur,
    //         amount.unwrap(),
    //         ExistenceRequirement::AllowDeath
    //     )
    // }

// Fund buyer escrow
// pub fn fund_buyer_escrow(from: T::AccountId, to: T::AccountId, value: u32) -> DispatchResult {
    
//     let sender_balance = Self::get_balance(&from);
//     let receiver_balance = Self::get_balance(&to);
  
//     // Calculate new balances
//     let updated_from_balance = sender_balance.checked_sub(value).ok_or(<Error<T>>::InsufficientFunds)?;
//     let updated_to_balance = receiver_balance.checked_add(value).expect("Entire supply fits in u32; qed");
  
//     // Write new balances to storage
//     <Balances<T>>::insert(&from, updated_from_balance);
//     <Balances<T>>::insert(&to, updated_to_balance);
  
//     Self::deposit_event(RawEvent::Transfer(from, to, value));
//     Ok(())
//   }
  



}



