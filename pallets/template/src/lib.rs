#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use frame_system::offchain::{AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer};
use sp_core::crypto::KeyTypeId;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"ocwd");
pub mod crypto {
	use super::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	app_crypto!(sr25519, KEY_TYPE);

	pub struct OcwAuthId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for OcwAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for OcwAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::{DispatchResult, OptionQuery, *};
	use frame_system::pallet_prelude::{*, OriginFor};
	use sp_std::{vec, vec::Vec};

	#[derive(Debug, TypeInfo, Decode, Encode)]
	pub struct ResourceInfo {
		pub cpu: u8,
		pub memory: u8,

		pub unused_cpu: u8,
		pub unused_memory: u8,
	}

	impl ResourceInfo {
		pub fn use_resource(&mut self, cpu: u8, memory: u8) {
			self.unused_cpu = self.unused_cpu - cpu;
			self.unused_memory = self.unused_memory - memory;
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
	}

	// The pallet's runtime storage items.
	// https://docs.substrate.io/main-docs/build/runtime-storage/
	#[pallet::storage]
	#[pallet::getter(fn something)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/main-docs/build/runtime-storage/#declaring-storage-items
	pub type Something<T> = StorageValue<_, u32>;

	#[pallet::storage]
	#[pallet::getter(fn resource)]
	pub(super) type Resource<T: Config> =
		StorageMap<_, Twox64Concat, u8, ResourceInfo, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn resource_needs)]
	// (cpu, memory)
	pub type ResourceNeeds<T> = StorageValue<_, (u8, u8), ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),

		ApplyResource(u8, u8, T::AccountId),

        ChargeResourceSuccess,

        RegisterResource(u8, u8 ,u8),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,

		ResourceNotFound,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn register_resource(origin: OriginFor<T>, cpu: u8, memory: u8, id: u8) -> DispatchResult {
            
            let _ = ensure_signed(origin)?;

            let resource = ResourceInfo {
               cpu,
               memory,
               unused_cpu: cpu,
               unused_memory: memory,
            };

            // 插入资源
            Resource::<T>::insert(id, resource);


            Self::deposit_event(Event::RegisterResource(id, cpu, memory));

            Ok(())
        }



		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn apply_resource(origin: OriginFor<T>, cpu: u8, memory: u8) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// 写入资源需求
			ResourceNeeds::<T>::set((cpu, memory));

			Self::deposit_event(Event::ApplyResource(cpu, memory, who));

			Ok(())
		}

		/// offchain worker 调用的函数
		/// 用于将数据上链
		#[pallet::weight(0)]
		pub fn charge_resource(
			origin: OriginFor<T>,
			cpu: u8,
			memory: u8,
			resource_id: u8,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			ensure!(Resource::<T>::contains_key(resource_id), Error::<T>::ResourceNotFound,);

			let mut resource_info = Resource::<T>::get(resource_id).unwrap();

			// 更新资源信息
			resource_info.use_resource(cpu, memory);

			// 重写资源
			Resource::<T>::insert(resource_id, resource_info);

            Self::deposit_event(Event::ChargeResourceSuccess);

			Ok(().into())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: T::BlockNumber) {
			log::info!("Hello World from offchain workers!: {:?}", block_number);

            // 为了展示效果，10个区块以后开始
            if block_number < 10u32.into() {
                return;
            }

			// let payload: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8];
			// _ = Self::send_signed_tx(payload);

			// 读取资源需求
			let (cpu, memory) = ResourceNeeds::<T>::get();

			// 分配 资源id， 假设恒定在 id = 1的资源上调度
			let resource_id = 1;

			// 回调交易，将数据上链
			_ = Self::send_signed_tx(cpu, memory, resource_id);

			log::info!("Leave from offchain workers!: {:?}", block_number);
		}

		// fn on_initialize(_n: T::BlockNumber) -> Weight {
		// 	log::info!("in on_initialize!");
		// 	0
		// }

		// fn on_finalize(_n: T::BlockNumber) {
		// 	log::info!("in on_finalize!");
		// }

		// fn on_idle(_n: T::BlockNumber, _remaining_weight: Weight) -> Weight {
		// 	log::info!("in on_idle!");
		// 	0
		// }
	}

	impl<T: Config> Pallet<T> {
		fn send_signed_tx(cpu: u8, memory: u8, resource_id: u8) -> Result<(), &'static str> {
			// 获取能签名的人
			let signer = Signer::<T, T::AuthorityId>::all_accounts();
			// 签名
			if !signer.can_sign() {
				return Err(
					"No local accounts available. Consider adding one via `author_insertKey` RPC.",
				)
			}

			// 发起带签名的交易
			let results = signer.send_signed_transaction(|_account| Call::charge_resource {
				cpu,
				memory,
				resource_id,
			});

			for (acc, res) in &results {
				match res {
					Ok(()) => log::info!(
						"[{:?}] resource need: cpu: {:?} memory: {:?}",
						acc.id,
						cpu,
						memory
					),
					Err(e) => log::error!("[{:?}] Failed to transaction: {:?}", acc.id, e),
				}
			}

			Ok(())
		}
	}
}
