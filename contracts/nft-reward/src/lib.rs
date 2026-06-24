#![cfg_attr(not(test), no_std)]
use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Env, Map, String, Symbol, Val, Vec,
};

/// Core display metadata for an NFT (title, description, image URI).
/// Supports off-chain storage references to keep gas costs low.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NftMetadata {
    pub title: String,
    pub description: String,
    pub image_uri: String,
    /// Hunt title at time of mint (for context/display).
    pub hunt_title: String,
    /// Rarity tier: 0 = default, 1 = common, 2 = uncommon, 3 = rare, 4 = epic, 5 = legendary.
    pub rarity: u32,
    /// Custom tier for special categories (0 = none).
    pub tier: u32,
}

/// Complete metadata returned by get_nft_metadata (includes NftData-derived fields).
#[contracttype]
#[derive(Clone, Debug)]
pub struct NftMetadataResponse {
    pub nft_id: u64,
    pub hunt_id: u64,
    pub hunt_title: String,
    pub completion_timestamp: u64,
    pub completion_player: Address,
    pub current_owner: Address,
    pub title: String,
    pub description: String,
    pub image_uri: String,
    pub rarity: u32,
    pub tier: u32,
}

/// NFT data structure stored on-chain.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NftData {
    pub nft_id: u64,
    pub hunt_id: u64,
    pub owner: Address,
    /// Player who completed the hunt (preserved after transfers).
    pub completion_player: Address,
    pub metadata: NftMetadata,
    pub minted_at: u64,
}

/// Event emitted when an NFT is minted.
#[contracttype]
#[derive(Clone, Debug)]
pub struct NftMintedEvent {
    pub nft_id: u64,
    pub hunt_id: u64,
    pub owner: Address,
    pub metadata: NftMetadata,
    pub minted_at: u64,
}

/// Event emitted when an NFT is transferred.
#[contracttype]
#[derive(Clone, Debug)]
pub struct NftTransferredEvent {
    pub nft_id: u64,
    pub from: Address,
    pub to: Address,
}

/// Event emitted when an NFT's mutable metadata is updated.
#[contracttype]
#[derive(Clone, Debug)]
pub struct NftMetadataUpdatedEvent {
    pub nft_id: u64,
    pub updater: Address,
}

mod errors;
mod storage;
use storage::Storage;

#[contract]
pub struct NftReward;

#[contractimpl]
impl NftReward {
    /// Mints a unique NFT as a reward for hunt completion.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment
    /// * `hunt_id` - The hunt this NFT commemorates
    /// * `player_address` - The address of the player completing the hunt (initial owner)
    /// * `metadata` - NFT metadata (title, description, image URI, hunt_title, rarity, tier)
    ///
    /// # Returns
    /// The unique NFT ID of the minted NFT
    pub fn mint_reward_nft(
        env: Env,
        hunt_id: u64,
        player_address: Address,
        metadata: NftMetadata,
    ) -> u64 {
        let minted_at = env.ledger().timestamp();

        let nft_id = Storage::next_nft_id(&env);

        let nft_data = NftData {
            nft_id,
            hunt_id,
            owner: player_address.clone(),
            completion_player: player_address.clone(),
            metadata: metadata.clone(),
            minted_at,
        };

        Storage::save_nft(&env, &nft_data);
        Storage::add_nft_to_owner(&env, &player_address, nft_id);

        let event = NftMintedEvent {
            nft_id,
            hunt_id,
            owner: player_address,
            metadata,
            minted_at,
        };
        env.events()
            .publish((Symbol::new(&env, "NftMinted"), nft_id), event);

        nft_id
    }

    /// Mints a reward NFT from a generic metadata map. This is the entrypoint
    /// used by cross-contract callers (e.g. RewardManager) that cannot depend
    /// on this crate's `NftMetadata` type directly.
    ///
    /// Expected keys in `metadata` (all optional, with sensible defaults):
    /// - "title": String
    /// - "description": String
    /// - "image_uri": String
    /// - "hunt_title": String (defaults to title when omitted/empty)
    /// - "rarity": u32
    /// - "tier": u32
    pub fn mint_reward_nft_from_map(
        env: Env,
        hunt_id: u64,
        player_address: Address,
        metadata: Map<Symbol, Val>,
    ) -> u64 {
        use soroban_sdk::TryFromVal;

        let title = metadata
            .get(Symbol::new(&env, "title"))
            .and_then(|v| String::try_from_val(&env, &v).ok())
            .unwrap_or_else(|| String::from_str(&env, ""));

        let description = metadata
            .get(Symbol::new(&env, "description"))
            .and_then(|v| String::try_from_val(&env, &v).ok())
            .unwrap_or_else(|| String::from_str(&env, ""));

        let image_uri = metadata
            .get(Symbol::new(&env, "image_uri"))
            .and_then(|v| String::try_from_val(&env, &v).ok())
            .unwrap_or_else(|| String::from_str(&env, ""));

        let hunt_title = metadata
            .get(Symbol::new(&env, "hunt_title"))
            .and_then(|v| String::try_from_val(&env, &v).ok())
            .unwrap_or_else(|| title.clone());

        let rarity = metadata
            .get(Symbol::new(&env, "rarity"))
            .and_then(|v| u32::try_from_val(&env, &v).ok())
            .unwrap_or(0u32);

        let tier = metadata
            .get(Symbol::new(&env, "tier"))
            .and_then(|v| u32::try_from_val(&env, &v).ok())
            .unwrap_or(0u32);

        let meta = NftMetadata {
            title,
            description,
            image_uri,
            hunt_title,
            rarity,
            tier,
        };

        Self::mint_reward_nft(env, hunt_id, player_address, meta)
    }

    /// Retrieves NFT data by ID.
    pub fn get_nft(env: Env, nft_id: u64) -> Option<NftData> {
        Storage::get_nft(&env, nft_id)
    }

    /// Returns complete metadata for an NFT, including hunt info and completion details.
    pub fn get_nft_metadata(env: Env, nft_id: u64) -> Option<NftMetadataResponse> {
        let nft = Storage::get_nft(&env, nft_id)?;
        Some(NftMetadataResponse {
            nft_id: nft.nft_id,
            hunt_id: nft.hunt_id,
            hunt_title: nft.metadata.hunt_title.clone(),
            completion_timestamp: nft.minted_at,
            completion_player: nft.completion_player.clone(),
            current_owner: nft.owner.clone(),
            title: nft.metadata.title.clone(),
            description: nft.metadata.description.clone(),
            image_uri: nft.metadata.image_uri.clone(),
            rarity: nft.metadata.rarity,
            tier: nft.metadata.tier,
        })
    }

    /// Updates mutable metadata fields (description, image_uri). Owner only.
    /// Title, hunt info, and attributes remain immutable for collectibility.
    pub fn update_nft_metadata(
        env: Env,
        nft_id: u64,
        updater: Address,
        new_description: String,
        new_image_uri: String,
    ) -> Result<(), crate::errors::NftErrorCode> {
        updater.require_auth();

        let mut nft = Storage::get_nft(&env, nft_id).ok_or(crate::errors::NftErrorCode::NftNotFound)?;

        if nft.owner != updater {
            return Err(crate::errors::NftErrorCode::NotOwner);
        }

        nft.metadata.description = new_description;
        nft.metadata.image_uri = new_image_uri;
        Storage::save_nft(&env, &nft);

        env.events().publish(
            (Symbol::new(&env, "NftMetadataUpdated"), nft_id),
            NftMetadataUpdatedEvent {
                nft_id,
                updater,
            },
        );

        Ok(())
    }

    /// Returns the total number of NFTs minted so far.
    pub fn total_supply(env: Env) -> u64 {
        Storage::get_nft_counter(&env)
    }

    /// Returns the owner of an NFT.
    pub fn owner_of(env: Env, nft_id: u64) -> Option<Address> {
        Storage::get_nft(&env, nft_id).map(|nft| nft.owner)
    }

    /// Alias for owner_of. Returns the owner of an NFT.
    pub fn get_nft_owner(env: Env, nft_id: u64) -> Option<Address> {
        Storage::get_nft(&env, nft_id).map(|nft| nft.owner)
    }

    /// Returns all NFT IDs owned by an address.
    pub fn get_player_nfts(env: Env, owner: Address) -> Vec<u64> {
        Storage::get_owner_nfts(&env, &owner)
    }

    /// Searches NFTs by title (case-insensitive partial match).
    /// Returns a vector of NFT IDs whose titles contain the search query.
    pub fn search_by_title(env: Env, query: String) -> Vec<u64> {
        let all_nft_ids = Storage::get_all_nft_ids(&env);
        let mut results = Vec::new(&env);
        
        let query_lower = {
            let mut lower = String::new(&env);
            for c in query.chars() {
                lower.push_char(c.to_ascii_lowercase());
            }
            lower
        };

        for nft_id in all_nft_ids.iter() {
            if let Some(nft) = Storage::get_nft(&env, nft_id) {
                let title_lower = {
                    let mut lower = String::new(&env);
                    for c in nft.metadata.title.chars() {
                        lower.push_char(c.to_ascii_lowercase());
                    }
                    lower
                };
                
                if title_lower.contains(&query_lower) {
                    results.push_back(nft_id);
                }
            }
        }
        
        results
    }

    /// Searches NFTs by hunt title (case-insensitive partial match).
    /// Returns a vector of NFT IDs whose hunt titles contain the search query.
    pub fn search_by_hunt_title(env: Env, query: String) -> Vec<u64> {
        let all_nft_ids = Storage::get_all_nft_ids(&env);
        let mut results = Vec::new(&env);
        
        let query_lower = {
            let mut lower = String::new(&env);
            for c in query.chars() {
                lower.push_char(c.to_ascii_lowercase());
            }
            lower
        };

        for nft_id in all_nft_ids.iter() {
            if let Some(nft) = Storage::get_nft(&env, nft_id) {
                let hunt_title_lower = {
                    let mut lower = String::new(&env);
                    for c in nft.metadata.hunt_title.chars() {
                        lower.push_char(c.to_ascii_lowercase());
                    }
                    lower
                };
                
                if hunt_title_lower.contains(&query_lower) {
                    results.push_back(nft_id);
                }
            }
        }
        
        results
    }

    /// Filters NFTs by rarity tier.
    /// Returns a vector of NFT IDs with the specified rarity.
    /// Rarity tiers: 0 = default, 1 = common, 2 = uncommon, 3 = rare, 4 = epic, 5 = legendary.
    pub fn search_by_rarity(env: Env, rarity: u32) -> Vec<u64> {
        let all_nft_ids = Storage::get_all_nft_ids(&env);
        let mut results = Vec::new(&env);

        for nft_id in all_nft_ids.iter() {
            if let Some(nft) = Storage::get_nft(&env, nft_id) {
                if nft.metadata.rarity == rarity {
                    results.push_back(nft_id);
                }
            }
        }
        
        results
    }

    /// Filters NFTs by custom tier.
    /// Returns a vector of NFT IDs with the specified tier.
    /// Tier: 0 = none, other values for custom categories.
    pub fn search_by_tier(env: Env, tier: u32) -> Vec<u64> {
        let all_nft_ids = Storage::get_all_nft_ids(&env);
        let mut results = Vec::new(&env);

        for nft_id in all_nft_ids.iter() {
            if let Some(nft) = Storage::get_nft(&env, nft_id) {
                if nft.metadata.tier == tier {
                    results.push_back(nft_id);
                }
            }
        }
        
        results
    }

    /// General search function with multiple metadata filters.
    /// All parameters are optional - NFTs must match all provided filters.
    /// 
    /// # Arguments
    /// * `title_query` - Optional partial match for NFT title (case-insensitive)
    /// * `hunt_title_query` - Optional partial match for hunt title (case-insensitive)
    /// * `rarity` - Optional rarity filter (exact match)
    /// * `tier` - Optional tier filter (exact match)
    /// 
    /// # Returns
    /// Vector of NFT IDs matching all provided filters
    pub fn search_nfts(
        env: Env,
        title_query: Option<String>,
        hunt_title_query: Option<String>,
        rarity: Option<u32>,
        tier: Option<u32>,
    ) -> Vec<u64> {
        let all_nft_ids = Storage::get_all_nft_ids(&env);
        let mut results = Vec::new(&env);

        let title_lower_opt = title_query.map(|q| {
            let mut lower = String::new(&env);
            for c in q.chars() {
                lower.push_char(c.to_ascii_lowercase());
            }
            lower
        });

        let hunt_title_lower_opt = hunt_title_query.map(|q| {
            let mut lower = String::new(&env);
            for c in q.chars() {
                lower.push_char(c.to_ascii_lowercase());
            }
            lower
        });

        for nft_id in all_nft_ids.iter() {
            if let Some(nft) = Storage::get_nft(&env, nft_id) {
                let mut matches = true;

                // Check title filter
                if let Some(ref query_lower) = title_lower_opt {
                    let title_lower = {
                        let mut lower = String::new(&env);
                        for c in nft.metadata.title.chars() {
                            lower.push_char(c.to_ascii_lowercase());
                        }
                        lower
                    };
                    if !title_lower.contains(query_lower) {
                        matches = false;
                    }
                }

                // Check hunt title filter
                if matches {
                    if let Some(ref query_lower) = hunt_title_lower_opt {
                        let hunt_title_lower = {
                            let mut lower = String::new(&env);
                            for c in nft.metadata.hunt_title.chars() {
                                lower.push_char(c.to_ascii_lowercase());
                            }
                            lower
                        };
                        if !hunt_title_lower.contains(query_lower) {
                            matches = false;
                        }
                    }
                }

                // Check rarity filter
                if matches {
                    if let Some(r) = rarity {
                        if nft.metadata.rarity != r {
                            matches = false;
                        }
                    }
                }

                // Check tier filter
                if matches {
                    if let Some(t) = tier {
                        if nft.metadata.tier != t {
                            matches = false;
                        }
                    }
                }

                if matches {
                    results.push_back(nft_id);
                }
            }
        }
        
        results
    }

    /// Transfers an NFT from one address to another.
    ///
    /// # Arguments
    /// * `nft_id` - The NFT to transfer
    /// * `from_address` - Current owner (must authorize the call)
    /// * `to_address` - New owner
    ///
    /// # Authorization
    /// The `from_address` must authorize this call via `require_auth`.
    /// For automatic transfers during reward distribution, the contract may be
    /// the `from_address` when invoked by an authorized party.
    pub fn transfer_nft(
        _env: Env,
        _nft_id: u64,
        _from_address: Address,
        _to_address: Address,
    ) -> Result<(), crate::errors::NftErrorCode> {
        Err(crate::errors::NftErrorCode::SoulboundNft)
    }
}

#[cfg(test)]
mod test;
