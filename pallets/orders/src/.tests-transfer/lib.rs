#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    dispatch::{DispatchError, DispatchResult},
    traits::{Get, Currency, ExistenceRequirement},
    debug,
};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;
use frame_system::{self as system, ensure_signed};

use pallet_permissions::StorefrontPermission;
use pallet_storefronts::{Module as Storefronts, Storefront, StorefrontById};
use pallet_utils::{Module as Utils, StorefrontId, WhoAndWhen, Content};
use pallet_products::{Module as Products, Product, ProductById, ProductId};

use core::convert::TryInto;
use std::convert::From;

pub mod functions;

pub type OrderId = u64;

// Order
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct Order<T: Trait> {
    pub id: OrderId,
    pub created: WhoAndWhen<T>,
    pub updated: Option<WhoAndWhen<T>>,
    pub owner: T::AccountId,
    pub order_state: OrderState,
    pub order_total: BalanceOf<T>,
    pub seller: T::AccountId,
    pub buyer_escrow: BalanceOf<T>,
    pub seller_escrow: BalanceOf<T>,
    pub storefront_id: StorefrontId,
    pub product_id: ProductId,
    pub content: Content,
}


#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, RuntimeDebug)]
pub enum OrderState {
    New,
    Pending,
    Accepted,
    Refused,
    Shipped,
    Complete,
    Refunded
}

impl Default for OrderState {
    fn default() -> Self {
        OrderState::New
    }
}


// Order update
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct OrderUpdate {
    pub content: Option<Content>,
    pub order_state: OrderState,
}


type BalanceOf<T> = <<T as pallet_utils::Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;


/// The pallet's configuration trait.
pub trait Trait: system::Trait
    + pallet_utils::Trait
    + pallet_storefronts::Trait
    + pallet_products::Trait
{
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type AfterOrderUpdated: AfterOrderUpdated<Self>;
}

#[impl_trait_for_tuples::impl_for_tuples(10)]
pub trait AfterOrderUpdated<T: Trait> {
    fn after_order_updated(account: T::AccountId, order: &Order<T>, old_data: OrderUpdate);
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
    {
        OrderCreated(AccountId, OrderId),
        OrderUpdated(AccountId, OrderId, OrderState),
        OrderDeleted(AccountId, OrderId),
        EscrowInitiated(AccountId, OrderId),
    }
);

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as OrderModule {
        pub NextOrderId get(fn next_order_id): OrderId = 1;

        pub OrderById get(fn order_by_id): map hasher(twox_64_concat) OrderId => Option<Order<T>>;

        pub OrderIdsByProductId get(fn order_ids_by_product_id):
             map hasher(twox_64_concat) ProductId => Vec<OrderId>;

        pub OrderIdsByStorefrontId get(fn product_ids_by_storefront_id):
             map hasher(twox_64_concat) StorefrontId => Vec<OrderId>;

        pub OrderIdsByAccount get(fn order_id_by_account):
             map hasher(twox_64_concat) T::AccountId => Vec<OrderId>;
             
    }
}


decl_error! {
    pub enum Error for Module<T: Trait> {
        // Order related errors:
        CannotCreateInHiddenScope,
        ProductIdNotFoundInGivenStorefront,
        CanNotOrderOwnProducts,
        OrderGivenSellerIsNotProductOwner,
        NoUpdatesForOrder,
        OrderNotFound,
        ProductNotFound,
        MustWaitSellerAcceptsOrder,
        MustWaitBuyerConfirmsShipment,
        OrderStateDoesNotExpectUpdate,
        OrderCompletionNotImplementedYet,
        InsufficientFunds,
    }
}


decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {

    // Initializing errors
    type Error = Error<T>;

    // Initializing events
    fn deposit_event() = default;


    
    // create order
    #[weight = 100_000 + T::DbWeight::get().reads_writes(8, 8)]
    pub fn create_order(
      origin,
      storefront_id: StorefrontId,
      product_id: ProductId,
      order_total: u64,
      seller: T::AccountId,
      buyer_escrow: u64,
      seller_escrow: u64,
      content: Content
    ) -> DispatchResult {
      let creator = ensure_signed(origin)?;

      Utils::<T>::is_valid_content(content.clone())?;

      let new_order_id = Self::next_order_id();
      let new_order: Order<T> = Order::new(new_order_id, creator.clone(), storefront_id, product_id, BalanceOf::<T>::from(order_total), seller, buyer_escrow, seller_escrow, content);

      // get the product by id
      let product = &mut Products::<T>::require_product(product_id)?;

      //get the storefront by id
      let storefront = &mut Storefronts::<T>::require_storefront(storefront_id)?;
      //Storefronts::<T>::ensure_storefront_exists(storefront_id)?; // double check ??
     

        // ensure given product_id belongs to the given store, if data incoherence, cancel order
        if let Some(product_storefront_id) = product.storefront_id {
        if storefront_id != product_storefront_id {
          //Storefronts::<T>::ensure_storefront_exists(storefront_id)?;
          return Err(Error::<T>::ProductIdNotFoundInGivenStorefront.into());
        }
    }
      // no order on hidden
      ensure!(!storefront.hidden, Error::<T>::CannotCreateInHiddenScope);
      ensure!(!product.hidden, Error::<T>::CannotCreateInHiddenScope);

      // check seller vs store/product owner
      let product_seller = &storefront.owner;
      ensure!(!storefront.is_owner(&creator), Error::<T>::CanNotOrderOwnProducts);
      if product_seller != &storefront.owner {
        return Err(Error::<T>::OrderGivenSellerIsNotProductOwner.into());
      }

      // simple escrow PoC

      //calc escrow buyer
      let buyer_escrow_total = (new_order.order_total + new_order.buyer_escrow);
      

      //T::Currency::transfer(&creator, &Utils::<T>::treasury_account(), BalanceOf::<T>::buyer_escrow_total.into(), ExistenceRequirement::KeepAlive);


     // <T as pallet_utils::Trait>::Currency::transfer(&creator, &Utils::<T>::treasury_account(), BalanceOf<T>::buyer_escrow_total.into(), ExistenceRequirement::KeepAlive);
     // Self::fund_buyer_escrow(&creator, &Utils::<T>::treasury_account(), buyer_escrow_total.into())?;

      <T as pallet_utils::Trait>::Currency::transfer(
        &creator,
        &Utils::<T>::treasury_account(),
        buyer_escrow_total.into(),
        //u64_to_balance_option(buyer_escrow_total), // .into() injecting u64 balance / price
        ExistenceRequirement::KeepAlive
      )?;

      //Self::deposit_event(RawEvent::EscrowInitiated(&creator, new_order_id));
       // order history
       OrderIdsByStorefrontId::mutate(storefront.id, |ids| ids.push(new_order_id));
       OrderIdsByProductId::mutate(product.id, |ids| ids.push(new_order_id));
       OrderById::<T>::insert(new_order_id, new_order);
       OrderIdsByAccount::<T>::mutate(&creator, |ids| ids.push(new_order_id));
    
      // TODO : Escrow !!
      // TODO : Alpha1 release : simple % lock ? based on OrderState ? Full-Auto ? Sudo?

      // 1 - Buyer buys -> OrderState::New -> buyer fund % locked somewhere
      // 2 - Seller is notified about new order 
      //         -> Accepts -> OrderState::Pending -> seller fund % locked somewhere
      //         -> or Refuses -> > OrderState::Canceled -> buyers funds % released
      // 3 - If accepted, seller ships order + proof of shipping -> OrderState::Shipped
      // 4 - Buyer receives order -> OrderState::Completed -> buyer and seller fund % released
      // 5 - Buyer receives nothing -> ?? think about moderation or just both funds are lost and goes in treasury account?


       // increment orderId
       NextOrderId::mutate(|n| { *n += 1; });

//       println!("Order created : {:#?}", new_order_id);
         debug::info!("Order created : {:?}", new_order_id);

      Self::deposit_event(RawEvent::OrderCreated(creator, new_order_id));
      Ok(())
    }




    
// ******* Update *********

#[weight = 100_000 + T::DbWeight::get().reads_writes(5, 3)]
 pub fn update_order(origin, order_id: OrderId, update: OrderUpdate) -> DispatchResult {

  let editor = ensure_signed(origin)?;

  let mut order = Self::order_by_id(order_id).ok_or(Error::<T>::OrderNotFound)?;
  let buyer_escrow_total = order.order_total + order.buyer_escrow;
  // let has_updates = update.order_state.match();
  // ensure!(has_updates, Error::<T>::NoUpdatesForOrder);



  ensure!(order.order_state != update.order_state, Error::<T>::NoUpdatesForOrder);

        if order.order_state == OrderState::New || order.order_state == OrderState::Pending {
        ensure!(!order.is_owner(&editor), Error::<T>::MustWaitSellerAcceptsOrder);
        // TODO : balances -> move 50% of order total from each buyer balance
      }
    
      else if order.order_state == OrderState::Shipped {
        ensure!(order.is_owner(&editor), Error::<T>::MustWaitBuyerConfirmsShipment);
      }
    
      else if update.order_state == OrderState::Complete {
       // TODO : release escrow / funds and seller gets 100% order total
          return Err(Error::<T>::OrderCompletionNotImplementedYet.into());
      }
    
      else if update.order_state == OrderState::Refused {
          // TODO : release escrow / funds for both
          //return Err(Error::<T>::OrderCompletionNotImplementedYet.into());
         // <T as pallet_utils::Trait>::Currency::transfer(&Utils::<T>::treasury_account(), &editor, BalanceOf::buyer_escrow_total.into(), ExistenceRequirement::KeepAlive);
      }
    
      // else {
      //   return Err(Error::<T>::OrderStateDoesNotExpectUpdate.into());
      // }
    
      let old_state = order.order_state;
     


       let mut old_data = OrderUpdate::default();

      if let Some(content) = update.content {
        if content != order.content {
        Utils::<T>::is_valid_content(content.clone())?;
        old_data.content = Some(order.content);
        old_data.order_state = old_state;
        order.content = content;
        order.order_state = update.order_state;
      }
    }
    order.order_state = update.order_state;
      order.updated = Some(WhoAndWhen::<T>::new(editor.clone()));

      debug::info!("Order updated : {:?}", order.order_state);

              <OrderById<T>>::insert(order.id, order.clone());
              T::AfterOrderUpdated::after_order_updated(editor.clone(), &order, old_data);
      
              Self::deposit_event(RawEvent::OrderUpdated(editor, order_id, order.order_state));


// #[weight = 100_000 + T::DbWeight::get().reads_writes(5, 3)]
// pub fn update_order(origin, order_id: OrderId, update: OrderUpdate) -> DispatchResult {
//   let editor = ensure_signed(origin)?;


//   let mut order = Self::require_order(order_id)?;
//   let mut order_state = order.order_state;

//   let has_updates =
//     update.order_state != order.order_state ||
//     update.content.is_some();

//   ensure!(has_updates, Error::<T>::NoUpdatesForOrder);


//   let mut is_update_applied = false;
//   let mut old_data = OrderUpdate::default();
//     // TODO : perms
//   // If  OrderState::New : only seller can update to accepted/pending or canceled/refund
//   // If  OrderState::Shipped : only buyer can update to received/complete

 

//   if let Some(content) = update.content {
//     if content != order.content {
//       Utils::<T>::is_valid_content(content.clone())?;
//       old_data.content = Some(order.content);
//       order.content = content;
//       is_update_applied = true;
//     }
//   }


 
//   if let OrderState<T> = update.order_state {
   
//       if order.order_state == OrderState::New {
//         ensure!(!order.is_owner(&editor), Error::<T>::MustWaitSellerAcceptsOrder);
//         // TODO : balances -> move 50% of order total from each buyer balance
//       }
    
//       else if order.order_state == OrderState::Shipped {
//         ensure!(order.is_owner(&editor), Error::<T>::MustWaitBuyerConfirmsShipment);
//       }
    
//       else if order.order_state == OrderState::Complete {
//           // TODO : release escrow / funds and seller gets 100% order total
//           return Err(Error::<T>::OrderCompletionNotImplementedYet.into());
//       }
    
//       else if order.order_state == OrderState::Refused {
//               // TODO : release escrow / funds for both
//               return Err(Error::<T>::OrderCompletionNotImplementedYet.into());
//       }
    
//       // else {
//       //   return Err(Error::<T>::OrderStateDoesNotExpectUpdate.into());
//       // }
    
    

//       old_data.order_state = order.order_state;
//       order.order_state = order_state;
//       is_update_applied = true;
//     }
  








//       // Update this product only if at least one field should be updated:
//       if is_update_applied {
//         order.updated = Some(WhoAndWhen::<T>::new(editor.clone()));


//         <OrderById<T>>::insert(order.id, order.clone());
//         T::AfterOrderUpdated::after_order_updated(editor.clone(), &order, old_data);

//         Self::deposit_event(RawEvent::OrderUpdated(editor, order_id));
//       }


  Ok(())

}







    } // decl_module



    
}



