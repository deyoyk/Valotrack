use std::{sync::Arc, time::Instant};

use reqwest::{header::CONTENT_TYPE, Method};
use serde_json::{json, Map, Value};

use crate::{app::AppState, content, riot::Session};

#[derive(Clone, Copy)]
struct Probe {
    id: &'static str,
    label: &'static str,
    category: &'static str,
    method: &'static str,
    path: &'static str,
    target: &'static str,
    status: &'static str,
    feature: Option<&'static str>,
    note: &'static str,
    arg: Option<&'static str>,
}

const AGENT_ITEM_TYPE: &str = "01bb38e1-da47-4e6a-9b3d-945fe4655707";

const PROBES: &[Probe] = &[
    // XMPP
    Probe {
        id: "xmpp_connection",
        label: "TCP XMPP Connection",
        category: "XMPP",
        method: "TCP",
        path: "xmpp://riot-chat",
        target: "realtime",
        status: "realtime",
        feature: None,
        note: "Riot's live chat transport. Valotrack reads the same presence and chat data through the authenticated local client instead of opening a second XMPP session.",
        arg: None,
    },
    // PVP
    Probe {
        id: "fetch_content",
        label: "Fetch Content",
        category: "PVP",
        method: "GET",
        path: "/content-service/v3/content?locale=en-US",
        target: "shared",
        status: "read",
        feature: None,
        note: "Live seasons, acts and content flags from Riot's shared content service.",
        arg: None,
    },
    Probe {
        id: "account_xp",
        label: "Account XP",
        category: "PVP",
        method: "GET",
        path: "/account-xp/v1/players/{puuid}",
        target: "pd",
        status: "read",
        feature: Some("account"),
        note: "Account level and XP progress for the signed-in player.",
        arg: None,
    },
    Probe {
        id: "player_loadout",
        label: "Player Loadout",
        category: "PVP",
        method: "GET",
        path: "/personalization/v3/players/{puuid}/playerloadout",
        target: "pd",
        status: "read",
        feature: Some("loadout"),
        note: "The player's equipped weapons, skins, sprays, cards and buddies.",
        arg: None,
    },
    Probe {
        id: "set_player_loadout",
        label: "Set Player Loadout",
        category: "PVP",
        method: "PUT",
        path: "/personalization/v3/players/{puuid}/playerloadout",
        target: "pd",
        status: "write",
        feature: Some("loadout"),
        note: "Changes the equipped loadout. Use the Loadout page, which validates ownership before writing.",
        arg: None,
    },
    Probe {
        id: "player_mmr",
        label: "Player MMR",
        category: "PVP",
        method: "GET",
        path: "/mmr/v1/players/{puuid}",
        target: "pd",
        status: "read",
        feature: Some("performance"),
        note: "Competitive rating, peak rank and per-act MMR history.",
        arg: None,
    },
    Probe {
        id: "match_history",
        label: "Match History",
        category: "PVP",
        method: "GET",
        path: "/match-history/v1/history/{puuid}?startIndex=0&endIndex=10",
        target: "pd",
        status: "read",
        feature: Some("performance"),
        note: "The player's recent match IDs and queue metadata.",
        arg: None,
    },
    Probe {
        id: "match_details",
        label: "Match Details",
        category: "PVP",
        method: "GET",
        path: "/match-details/v1/matches/{matchId}",
        target: "pd",
        status: "read",
        feature: Some("performance"),
        note: "Full scoreboard, rounds and economy for a match. Uses the newest match when no ID is supplied.",
        arg: Some("match"),
    },
    Probe {
        id: "competitive_updates",
        label: "Competitive Updates",
        category: "PVP",
        method: "GET",
        path: "/mmr/v1/players/{puuid}/competitiveupdates?startIndex=0&endIndex=20&queue=competitive",
        target: "pd",
        status: "read",
        feature: Some("performance"),
        note: "Recent ranked results with RR movement for each match.",
        arg: None,
    },
    Probe {
        id: "leaderboard",
        label: "Leaderboard",
        category: "PVP",
        method: "GET",
        path: "/mmr/v1/leaderboards/affinity/{shard}/queue/competitive/season/{season}",
        target: "pd",
        status: "read",
        feature: Some("leaderboard"),
        note: "Seasonal competitive leaderboard. The Leaderboard page resolves the active season automatically.",
        arg: None,
    },
    Probe {
        id: "penalties",
        label: "Penalties",
        category: "PVP",
        method: "GET",
        path: "/restrictions/v3/penalties",
        target: "pd",
        status: "read",
        feature: Some("account"),
        note: "Matchmaking restrictions and behavioral penalties on the account.",
        arg: None,
    },
    Probe {
        id: "config",
        label: "Config",
        category: "PVP",
        method: "GET",
        path: "/v1/config/{shard}",
        target: "pd",
        status: "read",
        feature: None,
        note: "Riot's remote client configuration and feature flags. Riot occasionally returns 503 for this service.",
        arg: None,
    },
    Probe {
        id: "name_service",
        label: "Name Service",
        category: "PVP",
        method: "PUT",
        path: "/name-service/v1/players",
        target: "pd",
        status: "write",
        feature: None,
        note: "Resolves or changes Riot IDs. Valotrack treats name changes as out of scope for safety.",
        arg: None,
    },
    // Party
    Probe {
        id: "party",
        label: "Party",
        category: "Party",
        method: "GET",
        path: "/parties/v1/parties/{partyId}",
        target: "glz",
        status: "read",
        feature: Some("matchmaking"),
        note: "Live party members, ready state, queue and accessibility.",
        arg: None,
    },
    Probe {
        id: "party_player",
        label: "Party Player",
        category: "Party",
        method: "GET",
        path: "/parties/v1/players/{puuid}",
        target: "glz",
        status: "read",
        feature: Some("matchmaking"),
        note: "Current party membership and pending invites for the player.",
        arg: None,
    },
    Probe {
        id: "remove_player",
        label: "Remove Player",
        category: "Party",
        method: "DELETE",
        path: "/parties/v1/parties/{partyId}/members/{memberPuuid}",
        target: "glz",
        status: "write",
        feature: None,
        note: "Owner-only removal of a party member. Not exposed from the read-only lab.",
        arg: None,
    },
    Probe {
        id: "set_ready",
        label: "Set Member Ready",
        category: "Party",
        method: "POST",
        path: "/parties/v1/parties/{partyId}/members/{puuid}/setReady",
        target: "glz",
        status: "write",
        feature: Some("matchmaking"),
        note: "Toggles ready state. The Matchmaking page requires live controls and supports dry-run.",
        arg: None,
    },
    Probe {
        id: "refresh_competitive_tier",
        label: "Refresh Competitive Tier",
        category: "Party",
        method: "POST",
        path: "/parties/v1/parties/{partyId}/refreshCompetitiveTier",
        target: "glz",
        status: "write",
        feature: None,
        note: "Asks Riot to refresh party rank badges. Safe but mutating, so it is listed for visibility.",
        arg: None,
    },
    Probe {
        id: "refresh_player_identity",
        label: "Refresh Player Identity",
        category: "Party",
        method: "POST",
        path: "/parties/v1/parties/{partyId}/refreshPlayerIdentity",
        target: "glz",
        status: "write",
        feature: None,
        note: "Refreshes player cards, titles and account levels across the party.",
        arg: None,
    },
    Probe {
        id: "refresh_pings",
        label: "Refresh Pings",
        category: "Party",
        method: "POST",
        path: "/parties/v1/parties/{partyId}/refreshPings",
        target: "glz",
        status: "write",
        feature: None,
        note: "Refreshes latency information shown by the Riot client.",
        arg: None,
    },
    Probe {
        id: "change_queue",
        label: "Change Queue",
        category: "Party",
        method: "POST",
        path: "/parties/v1/parties/{partyId}/queue",
        target: "glz",
        status: "write",
        feature: Some("matchmaking"),
        note: "Changes the party's selected queue. The Matchmaking page validates eligibility.",
        arg: None,
    },
    Probe {
        id: "start_custom_game",
        label: "Start Custom Game",
        category: "Party",
        method: "POST",
        path: "/parties/v1/parties/{partyId}/customGame/start",
        target: "glz",
        status: "write",
        feature: None,
        note: "Starts a custom lobby. Not exposed from the read-only lab.",
        arg: None,
    },
    Probe {
        id: "enter_queue",
        label: "Enter Matchmaking Queue",
        category: "Party",
        method: "POST",
        path: "/parties/v1/parties/{partyId}/matchmaking/join",
        target: "glz",
        status: "write",
        feature: Some("matchmaking"),
        note: "Starts matchmaking. Guarded by the live-controls setting on the Matchmaking page.",
        arg: None,
    },
    Probe {
        id: "leave_queue",
        label: "Leave Matchmaking Queue",
        category: "Party",
        method: "POST",
        path: "/parties/v1/parties/{partyId}/matchmaking/leave",
        target: "glz",
        status: "write",
        feature: Some("matchmaking"),
        note: "Cancels matchmaking. Guarded by the live-controls setting on the Matchmaking page.",
        arg: None,
    },
    Probe {
        id: "set_accessibility",
        label: "Set Party Accessibility",
        category: "Party",
        method: "POST",
        path: "/parties/v1/parties/{partyId}/accessibility",
        target: "glz",
        status: "write",
        feature: Some("matchmaking"),
        note: "Opens or closes the party. Exposed with dry-run on the Matchmaking page.",
        arg: None,
    },
    Probe {
        id: "set_custom_game_settings",
        label: "Set Custom Game Settings",
        category: "Party",
        method: "POST",
        path: "/parties/v1/parties/{partyId}/customGame/settings",
        target: "glz",
        status: "write",
        feature: None,
        note: "Changes custom lobby rules. Not exposed from the read-only lab.",
        arg: None,
    },
    Probe {
        id: "party_invite",
        label: "Party Invite",
        category: "Party",
        method: "POST",
        path: "/parties/v1/parties/{partyId}/invites/name/{name}/tag/{tag}",
        target: "glz",
        status: "write",
        feature: Some("matchmaking"),
        note: "Invites a Riot ID. The Matchmaking page supports dry-run and live controls.",
        arg: None,
    },
    Probe {
        id: "party_request",
        label: "Party Request",
        category: "Party",
        method: "POST",
        path: "/parties/v1/parties/{partyId}/request",
        target: "glz",
        status: "write",
        feature: None,
        note: "Requests to join a party. Not exposed from the read-only lab.",
        arg: None,
    },
    Probe {
        id: "party_decline",
        label: "Party Decline",
        category: "Party",
        method: "POST",
        path: "/parties/v1/parties/{partyId}/invites/{puuid}/decline",
        target: "glz",
        status: "write",
        feature: None,
        note: "Declines a party invite. Not exposed from the read-only lab.",
        arg: None,
    },
    Probe {
        id: "custom_game_configs",
        label: "Custom Game Configs",
        category: "Party",
        method: "GET",
        path: "/parties/v1/customgames/configs",
        target: "glz",
        status: "legacy",
        feature: None,
        note: "The current Riot client returns 404 for this legacy route; custom settings are served inside the party object.",
        arg: None,
    },
    Probe {
        id: "party_chat_token",
        label: "Party Chat Token",
        category: "Party",
        method: "GET",
        path: "/parties/v1/parties/{partyId}/chatToken",
        target: "glz",
        status: "legacy",
        feature: None,
        note: "The current Riot client returns 404; chat authorization is handled by the local chat service.",
        arg: None,
    },
    Probe {
        id: "party_voice_token",
        label: "Party Voice Token",
        category: "Party",
        method: "GET",
        path: "/parties/v1/parties/{partyId}/voicetoken",
        target: "glz",
        status: "legacy",
        feature: None,
        note: "The current Riot client returns 404; voice provisioning is handled internally by Riot.",
        arg: None,
    },
    Probe {
        id: "disable_code",
        label: "Party Disable Code",
        category: "Party",
        method: "DELETE",
        path: "/parties/v1/parties/{partyId}/invitecode",
        target: "glz",
        status: "write",
        feature: None,
        note: "Revokes the current party invite code. Not exposed from the read-only lab.",
        arg: None,
    },
    Probe {
        id: "generate_code",
        label: "Party Generate Code",
        category: "Party",
        method: "POST",
        path: "/parties/v1/parties/{partyId}/invitecode",
        target: "glz",
        status: "write",
        feature: Some("matchmaking"),
        note: "Generates a party invite code. The Matchmaking page supports dry-run and live controls.",
        arg: None,
    },
    Probe {
        id: "join_by_code",
        label: "Party Join By Code",
        category: "Party",
        method: "POST",
        path: "/parties/v1/parties/joinbycode/{code}",
        target: "glz",
        status: "write",
        feature: Some("matchmaking"),
        note: "Joins a party by invite code. The Matchmaking page supports dry-run and live controls.",
        arg: None,
    },
    // Store
    Probe {
        id: "prices",
        label: "Prices",
        category: "Store",
        method: "GET",
        path: "/store/v1/offers/",
        target: "pd",
        status: "legacy",
        feature: None,
        note: "The current PD shard returns 404 for this legacy route; storefront offers already include pricing.",
        arg: None,
    },
    Probe {
        id: "storefront",
        label: "Storefront",
        category: "Store",
        method: "POST",
        path: "/store/v3/storefront/{puuid}",
        target: "pd",
        status: "read",
        feature: Some("store"),
        note: "Daily shop, featured bundles, night market and remaining purchase seconds.",
        arg: None,
    },
    Probe {
        id: "wallet",
        label: "Wallet",
        category: "Store",
        method: "GET",
        path: "/store/v1/wallet/{puuid}",
        target: "pd",
        status: "read",
        feature: Some("store"),
        note: "VP, Radianite and other currency balances.",
        arg: None,
    },
    Probe {
        id: "owned_items",
        label: "Owned Items",
        category: "Store",
        method: "GET",
        path: "/store/v1/entitlements/{puuid}/{agentItemType}",
        target: "pd",
        status: "read",
        feature: Some("matchmaking"),
        note: "Agent entitlements for this account. This is the same source used to filter instalock choices.",
        arg: None,
    },
    // Pre-Game
    Probe {
        id: "pregame_player",
        label: "Pre-Game Player",
        category: "Pre-Game",
        method: "GET",
        path: "/pregame/v1/players/{puuid}",
        target: "glz",
        status: "read",
        feature: None,
        note: "The player's current agent-select match, if one is active.",
        arg: None,
    },
    Probe {
        id: "pregame_match",
        label: "Pre-Game Match",
        category: "Pre-Game",
        method: "GET",
        path: "/pregame/v1/matches/{pregameMatchId}",
        target: "glz",
        status: "read",
        feature: Some("live"),
        note: "Agent-select lobby, teams, picks and lock state.",
        arg: None,
    },
    Probe {
        id: "pregame_loadouts",
        label: "Pre-Game Loadouts",
        category: "Pre-Game",
        method: "GET",
        path: "/pregame/v1/matches/{pregameMatchId}/loadouts",
        target: "glz",
        status: "read",
        feature: Some("live"),
        note: "Weapon and skin loadouts for everyone in agent select.",
        arg: None,
    },
    Probe {
        id: "select_character",
        label: "Select Character",
        category: "Pre-Game",
        method: "POST",
        path: "/pregame/v1/matches/{pregameMatchId}/select/{agentId}",
        target: "glz",
        status: "write",
        feature: Some("matchmaking"),
        note: "Hovers an agent. Instalock validates ownership and supports dry-run.",
        arg: None,
    },
    Probe {
        id: "lock_character",
        label: "Lock Character",
        category: "Pre-Game",
        method: "POST",
        path: "/pregame/v1/matches/{pregameMatchId}/lock/{agentId}",
        target: "glz",
        status: "write",
        feature: Some("matchmaking"),
        note: "Locks an agent. Instalock validates ownership and supports dry-run.",
        arg: None,
    },
    Probe {
        id: "pregame_quit",
        label: "Pre-Game Quit",
        category: "Pre-Game",
        method: "POST",
        path: "/pregame/v1/matches/{pregameMatchId}/quit",
        target: "glz",
        status: "write",
        feature: Some("matchmaking"),
        note: "Dodges agent select. The Matchmaking page requires live controls and supports dry-run.",
        arg: None,
    },
    // Current Game
    Probe {
        id: "current_game_player",
        label: "Current Game Player",
        category: "Current Game",
        method: "GET",
        path: "/core-game/v1/players/{puuid}",
        target: "glz",
        status: "read",
        feature: None,
        note: "The player's live match, if one is in progress.",
        arg: None,
    },
    Probe {
        id: "current_game_match",
        label: "Current Game Match",
        category: "Current Game",
        method: "GET",
        path: "/core-game/v1/matches/{coreMatchId}",
        target: "glz",
        status: "read",
        feature: Some("live"),
        note: "Live score, round state and player roster.",
        arg: None,
    },
    Probe {
        id: "current_game_loadouts",
        label: "Current Game Loadouts",
        category: "Current Game",
        method: "GET",
        path: "/core-game/v1/matches/{coreMatchId}/loadouts",
        target: "glz",
        status: "read",
        feature: Some("live"),
        note: "Loadouts for every player in the live match.",
        arg: None,
    },
    Probe {
        id: "current_game_quit",
        label: "Current Game Quit",
        category: "Current Game",
        method: "POST",
        path: "/core-game/v1/matches/{coreMatchId}/quit",
        target: "glz",
        status: "write",
        feature: None,
        note: "Leaves a live match. Intentionally not exposed because it affects the whole team and ranked standing.",
        arg: None,
    },
    // Contracts
    Probe {
        id: "item_upgrades",
        label: "Item Upgrades",
        category: "Contracts",
        method: "GET",
        path: "/contract-definitions/v3/item-upgrades",
        target: "pd",
        status: "read",
        feature: Some("contracts"),
        note: "Upgrade definitions for owned weapon skins and their Radianite costs.",
        arg: None,
    },
    Probe {
        id: "contracts",
        label: "Contracts",
        category: "Contracts",
        method: "GET",
        path: "/contracts/v1/contracts/{puuid}",
        target: "pd",
        status: "read",
        feature: Some("contracts"),
        note: "Agent contracts, mission progress and active contract state.",
        arg: None,
    },
    Probe {
        id: "activate_contract",
        label: "Activate Contract",
        category: "Contracts",
        method: "POST",
        path: "/contracts/v1/contracts/{puuid}/special/{contractId}",
        target: "pd",
        status: "write",
        feature: Some("contracts"),
        note: "Activates an agent contract. The Contracts page supports dry-run.",
        arg: None,
    },
    // Local
    Probe {
        id: "local_help",
        label: "Local Help",
        category: "Local",
        method: "GET",
        path: "/help",
        target: "local",
        status: "read",
        feature: None,
        note: "Every event, function and type the local Riot client exposes.",
        arg: None,
    },
    Probe {
        id: "sessions",
        label: "Sessions",
        category: "Local",
        method: "GET",
        path: "/product-session/v1/external-sessions",
        target: "local",
        status: "read",
        feature: None,
        note: "Running Riot products, launch arguments and patchline.",
        arg: None,
    },
    Probe {
        id: "rso_user_info",
        label: "RSO User Info",
        category: "Local",
        method: "GET",
        path: "/rso-auth/v1/authorization/userinfo",
        target: "local",
        status: "read",
        feature: None,
        note: "Riot account identity, country and locale from the local session.",
        arg: None,
    },
    Probe {
        id: "client_region",
        label: "Client Region",
        category: "Local",
        method: "GET",
        path: "/riotclient/region-locale",
        target: "local",
        status: "read",
        feature: None,
        note: "The Riot client's selected region and locale.",
        arg: None,
    },
    Probe {
        id: "account_alias",
        label: "Account Alias",
        category: "Local",
        method: "GET",
        path: "/player-account/aliases/v1/active",
        target: "local",
        status: "read",
        feature: Some("account"),
        note: "The signed-in account's game name and tag line.",
        arg: None,
    },
    Probe {
        id: "entitlements_token",
        label: "Entitlements Token",
        category: "Local",
        method: "GET",
        path: "/entitlements/v1/token",
        target: "local",
        status: "read",
        feature: None,
        note: "Local access and entitlement tokens. Tokens are redacted in the response view.",
        arg: None,
    },
    Probe {
        id: "chat_session",
        label: "Chat Session",
        category: "Local",
        method: "GET",
        path: "/chat/v1/session",
        target: "local",
        status: "read",
        feature: Some("social"),
        note: "The active chat identity, region and session state.",
        arg: None,
    },
    Probe {
        id: "friends",
        label: "Friends",
        category: "Local",
        method: "GET",
        path: "/chat/v4/friends",
        target: "local",
        status: "read",
        feature: Some("social"),
        note: "The full friends list, groups and last-online data.",
        arg: None,
    },
    Probe {
        id: "send_friend_request",
        label: "Send Friend Request",
        category: "Local",
        method: "POST",
        path: "/chat/v4/friendrequests",
        target: "local",
        status: "write",
        feature: Some("social"),
        note: "Sends a friend request by Riot ID. Available on the Social page.",
        arg: None,
    },
    Probe {
        id: "remove_friend_request",
        label: "Remove Friend Request",
        category: "Local",
        method: "DELETE",
        path: "/chat/v4/friendrequests",
        target: "local",
        status: "write",
        feature: Some("social"),
        note: "Removes a pending friend request. Available on the Social page.",
        arg: None,
    },
    Probe {
        id: "presence",
        label: "Presence",
        category: "Local",
        method: "GET",
        path: "/chat/v4/presences",
        target: "local",
        status: "read",
        feature: Some("social"),
        note: "Live presence, queue, map and party information for friends.",
        arg: None,
    },
    Probe {
        id: "friend_requests",
        label: "Friend Requests",
        category: "Local",
        method: "GET",
        path: "/chat/v4/friendrequests",
        target: "local",
        status: "read",
        feature: Some("social"),
        note: "Incoming and outgoing friend requests.",
        arg: None,
    },
    Probe {
        id: "local_swagger_docs",
        label: "Local Swagger Docs",
        category: "Local",
        method: "GET",
        path: "/swagger/v3/openapi.json",
        target: "local",
        status: "read",
        feature: None,
        note: "OpenAPI description of the local Riot client endpoints.",
        arg: None,
    },
    Probe {
        id: "local_websocket",
        label: "Local WebSocket",
        category: "Local",
        method: "WSS",
        path: "wss://127.0.0.1:{port}",
        target: "realtime",
        status: "realtime",
        feature: None,
        note: "Live event stream. Valotrack polls the local client and uses its state cache instead of holding a second socket.",
        arg: None,
    },
    // Local Chat
    Probe {
        id: "party_chat_info",
        label: "Party Chat Info",
        category: "Local Chat",
        method: "GET",
        path: "/chat/v6/conversations/ares-parties",
        target: "local",
        status: "read",
        feature: Some("social"),
        note: "Party conversation metadata.",
        arg: None,
    },
    Probe {
        id: "pregame_chat_info",
        label: "Pre-Game Chat Info",
        category: "Local Chat",
        method: "GET",
        path: "/chat/v6/conversations/ares-pregame",
        target: "local",
        status: "read",
        feature: Some("live"),
        note: "Agent-select team chat conversation metadata.",
        arg: None,
    },
    Probe {
        id: "current_game_chat_info",
        label: "Current Game Chat Info",
        category: "Local Chat",
        method: "GET",
        path: "/chat/v6/conversations/ares-coregame",
        target: "local",
        status: "read",
        feature: Some("live"),
        note: "Live match team and all-chat conversation metadata.",
        arg: None,
    },
    Probe {
        id: "all_chat_info",
        label: "All Chat Info",
        category: "Local Chat",
        method: "GET",
        path: "/chat/v6/conversations",
        target: "local",
        status: "read",
        feature: Some("social"),
        note: "Every active conversation, including DMs, party and match chat.",
        arg: None,
    },
    Probe {
        id: "chat_participants",
        label: "Chat Participants",
        category: "Local Chat",
        method: "GET",
        path: "/chat/v5/participants",
        target: "local",
        status: "read",
        feature: Some("social"),
        note: "Participant identity mapping for every conversation.",
        arg: None,
    },
    Probe {
        id: "send_chat",
        label: "Send Chat",
        category: "Local Chat",
        method: "POST",
        path: "/chat/v6/messages",
        target: "local",
        status: "write",
        feature: Some("social"),
        note: "Sends a whisper or group message through Riot chat. Available on the Social page.",
        arg: None,
    },
    Probe {
        id: "chat_history",
        label: "Chat History",
        category: "Local Chat",
        method: "GET",
        path: "/chat/v6/messages?cid={cid}",
        target: "local",
        status: "read",
        feature: Some("social"),
        note: "Messages for a conversation. Defaults to the newest active conversation.",
        arg: Some("cid"),
    },
    // Authentication
    Probe {
        id: "auth_cookies",
        label: "Auth Cookies",
        category: "Authentication",
        method: "POST",
        path: "https://auth.riotgames.com/api/v1/authorization",
        target: "reauth",
        status: "reauth",
        feature: None,
        note: "Starts a fresh username/password login. Valotrack deliberately uses the already signed-in local Riot client.",
        arg: None,
    },
    Probe {
        id: "auth_request",
        label: "Auth Request",
        category: "Authentication",
        method: "PUT",
        path: "https://auth.riotgames.com/api/v1/authorization",
        target: "reauth",
        status: "reauth",
        feature: None,
        note: "Completes a login/MFA challenge. Requires credentials that Valotrack never stores.",
        arg: None,
    },
    Probe {
        id: "mfa",
        label: "Multi-Factor Authentication",
        category: "Authentication",
        method: "PUT",
        path: "https://auth.riotgames.com/api/v1/mfa",
        target: "reauth",
        status: "reauth",
        feature: None,
        note: "Submits an MFA code. Requires an in-progress Riot login challenge.",
        arg: None,
    },
    Probe {
        id: "cookie_reauth",
        label: "Cookie Reauth",
        category: "Authentication",
        method: "GET",
        path: "https://auth.riotgames.com/authorize",
        target: "reauth",
        status: "reauth",
        feature: None,
        note: "Refreshes a remote session with saved Riot cookies. Valotrack never stores Riot cookies.",
        arg: None,
    },
    Probe {
        id: "entitlement",
        label: "Entitlement",
        category: "Authentication",
        method: "POST",
        path: "https://entitlements.auth.riotgames.com/api/token/v1",
        target: "auth",
        status: "read",
        feature: None,
        note: "Exchanges the local access token for an entitlement JWT. Redacted in the response view.",
        arg: None,
    },
    Probe {
        id: "player_info",
        label: "Player Info",
        category: "Authentication",
        method: "GET",
        path: "https://auth.riotgames.com/userinfo",
        target: "auth",
        status: "read",
        feature: None,
        note: "Riot account country, locale, verification state and game name.",
        arg: None,
    },
    Probe {
        id: "riot_geo",
        label: "Riot Geo",
        category: "Authentication",
        method: "PUT",
        path: "https://riot-geo.pas.si.riotgames.com/pas/v1/product/valorant",
        target: "reauth",
        status: "reauth",
        feature: None,
        note: "Resolves a region from a remote ID token. The local client already provides the shard and affinity.",
        arg: None,
    },
    Probe {
        id: "pas_token",
        label: "PAS Token",
        category: "Authentication",
        method: "GET",
        path: "https://riot-geo.pas.si.riotgames.com/pas/v1/service/chat",
        target: "auth",
        status: "read",
        feature: None,
        note: "Chat PAS token with the XMPP affinity. Redacted in the response view.",
        arg: None,
    },
    Probe {
        id: "riot_client_config",
        label: "Riot Client Config",
        category: "Authentication",
        method: "GET",
        path: "https://clientconfig.rpg.riotgames.com/api/v1/config/player?app=Riot%20Client",
        target: "clientconfig",
        status: "read",
        feature: None,
        note: "Remote Riot client configuration. Valotrack shows only chat affinity keys to keep the view readable.",
        arg: None,
    },
];

fn endpoint_json(probe: &Probe) -> Value {
    json!({
        "id": probe.id,
        "label": probe.label,
        "category": probe.category,
        "method": probe.method,
        "path": probe.path,
        "target": probe.target,
        "status": probe.status,
        "feature": probe.feature,
        "note": probe.note,
        "arg": probe.arg
    })
}

pub async fn catalog(state: &Arc<AppState>) -> Value {
    let settings = state.settings_snapshot();
    let session = state.riot.session(&settings.region).await.ok();
    let context = match session {
        Some(session) => {
            let account = content::account_label(&session).await;
            json!({
                "available": true,
                "account": account,
                "puuid": session.puuid,
                "shard": session.region.shard,
                "region": session.region.glz_a,
                "glz": session.region.glz_host(),
                "clientVersion": session.client_version
            })
        }
        None => json!({
            "available": false,
            "account": Value::Null,
            "puuid": Value::Null,
            "shard": Value::Null,
            "region": Value::Null,
            "glz": Value::Null,
            "clientVersion": Value::Null
        }),
    };
    let endpoints = PROBES.iter().map(endpoint_json).collect::<Vec<_>>();
    json!({
        "endpoints": endpoints,
        "context": context,
        "count": endpoints.len(),
        "categories": ["PVP", "Party", "Store", "Pre-Game", "Current Game", "Contracts", "Local", "Local Chat", "Authentication", "XMPP"]
    })
}

async fn party_id(session: &Session) -> Option<String> {
    session
        .glz_get(&format!("/parties/v1/players/{}", session.puuid))
        .await
        .ok()
        .and_then(|value| {
            value
                .get("CurrentPartyID")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .map(str::to_string)
        })
}

async fn pregame_match_id(session: &Session) -> Option<String> {
    session
        .glz_get(&format!("/pregame/v1/players/{}", session.puuid))
        .await
        .ok()
        .and_then(|value| {
            value
                .get("MatchID")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .map(str::to_string)
        })
}

async fn core_match_id(session: &Session) -> Option<String> {
    session
        .glz_get(&format!("/core-game/v1/players/{}", session.puuid))
        .await
        .ok()
        .and_then(|value| {
            value
                .get("MatchID")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .map(str::to_string)
        })
}

async fn latest_match_id(session: &Session) -> Option<String> {
    if let Ok(value) = session
        .pd_get(&format!(
            "/match-history/v1/history/{}?startIndex=0&endIndex=1",
            session.puuid
        ))
        .await
    {
        if let Some(id) = value
            .get("History")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("MatchID"))
            .and_then(Value::as_str)
        {
            return Some(id.to_string());
        }
    }
    session
        .pd_get(&format!(
            "/mmr/v1/players/{}/competitiveupdates?startIndex=0&endIndex=1&queue=competitive",
            session.puuid
        ))
        .await
        .ok()
        .and_then(|value| {
            value
                .get("Matches")
                .and_then(Value::as_array)
                .and_then(|rows| rows.first())
                .and_then(|row| row.get("MatchID"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

async fn first_conversation(session: &Session) -> Option<String> {
    session
        .local_get("/chat/v6/conversations")
        .await
        .ok()
        .and_then(|value| {
            value
                .get("conversations")
                .and_then(Value::as_array)
                .and_then(|rows| rows.first())
                .and_then(|row| row.get("cid"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

async fn resolve_path(
    session: &Session,
    probe: &Probe,
    arg: Option<&str>,
) -> Result<String, String> {
    let mut path = probe.path.to_string();
    path = path.replace("{puuid}", &session.puuid);
    path = path.replace("{shard}", &session.region.shard);
    path = path.replace("{agentItemType}", AGENT_ITEM_TYPE);

    if path.contains("{partyId}") {
        let id = party_id(session)
            .await
            .ok_or_else(|| "You are not in a party right now.".to_string())?;
        path = path.replace("{partyId}", &id);
    }
    if path.contains("{pregameMatchId}") {
        let id = match arg.filter(|value| !value.trim().is_empty()) {
            Some(value) => value.trim().to_string(),
            None => pregame_match_id(session)
                .await
                .ok_or_else(|| "No agent-select match is active.".to_string())?,
        };
        path = path.replace("{pregameMatchId}", &id);
    }
    if path.contains("{coreMatchId}") {
        let id = match arg.filter(|value| !value.trim().is_empty()) {
            Some(value) => value.trim().to_string(),
            None => core_match_id(session)
                .await
                .ok_or_else(|| "No live match is active.".to_string())?,
        };
        path = path.replace("{coreMatchId}", &id);
    }
    if path.contains("{matchId}") {
        let id = match arg.filter(|value| !value.trim().is_empty()) {
            Some(value) => value.trim().to_string(),
            None => latest_match_id(session)
                .await
                .ok_or_else(|| "No recent match was found.".to_string())?,
        };
        path = path.replace("{matchId}", &id);
    }
    if path.contains("{season}") {
        let seasons = session.seasons().await;
        let season = Session::active_season(&seasons)
            .ok_or_else(|| "Could not resolve the active competitive season.".to_string())?;
        path = path.replace("{season}", &season);
    }
    if path.contains("{cid}") {
        let id = match arg.filter(|value| !value.trim().is_empty()) {
            Some(value) => value.trim().to_string(),
            None => first_conversation(session)
                .await
                .ok_or_else(|| "No conversation is available.".to_string())?,
        };
        path = path.replace("{cid}", &urlencoding::encode(&id));
    }
    Ok(path)
}

fn redact_string(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    format!("[redacted {} chars]", value.chars().count())
}

fn redact(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut clean = Map::new();
            for (key, value) in map {
                let lower = key.to_lowercase();
                let secret = lower.contains("token")
                    || lower == "ssid"
                    || lower == "privatejwt"
                    || lower.ends_with("_jwt");
                if secret {
                    clean.insert(
                        key,
                        match value {
                            Value::String(text) => json!(redact_string(&text)),
                            Value::Null => Value::Null,
                            other => json!(format!("[redacted {}]", other)),
                        },
                    );
                } else {
                    clean.insert(key, redact(value));
                }
            }
            Value::Object(clean)
        }
        Value::Array(rows) => Value::Array(rows.into_iter().map(redact).collect()),
        other => other,
    }
}

fn client_config_summary(value: &Value) -> Value {
    let mut summary = Map::new();
    for key in ["chat.affinities", "chat.affinity_domains", "chat.port"] {
        if let Some(value) = value.get(key) {
            summary.insert(key.to_string(), value.clone());
        }
    }
    summary.insert(
        "keys".to_string(),
        json!(value.as_object().map(Map::len).unwrap_or(0)),
    );
    Value::Object(summary)
}

async fn remote_request(
    session: &Session,
    method: Method,
    url: &str,
    body: Option<&Value>,
) -> Result<Value, String> {
    let mut request = session
        .http
        .request(method, url)
        .headers(session.headers.clone())
        .header(CONTENT_TYPE, "application/json");
    if let Some(body) = body {
        request = request.json(body);
    }
    let response = request.send().await.map_err(|error| error.to_string())?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "HTTP {}: {}",
            status.as_u16(),
            text.chars().take(240).collect::<String>()
        ));
    }
    Ok(serde_json::from_str::<Value>(&text).unwrap_or_else(|_| {
        json!({
            "authToken": text.trim(),
            "length": text.trim().chars().count()
        })
    }))
}

pub async fn probe(
    state: &Arc<AppState>,
    id: &str,
    arg: Option<&str>,
) -> Result<Value, String> {
    let probe = PROBES
        .iter()
        .find(|probe| probe.id == id)
        .ok_or_else(|| "Unknown endpoint.".to_string())?;
    if probe.status != "read" {
        return Ok(json!({
            "ok": false,
            "status": probe.status,
            "method": probe.method,
            "path": probe.path,
            "target": probe.target,
            "note": probe.note,
            "error": match probe.status {
                "reauth" => "This endpoint needs Riot credentials or cookies, which Valotrack does not store.",
                "realtime" => "This is a live socket or transport, not a request that can be replayed here.",
                "write" => "This is a mutating endpoint. Use its dedicated page with live controls and dry-run.",
                "legacy" => "The current Riot client no longer serves this route.",
                _ => "This endpoint is listed for coverage but is not callable from the lab.",
            }
        }));
    }

    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let started = Instant::now();
    let path = match resolve_path(&session, probe, arg).await {
        Ok(path) => path,
        Err(error) => {
            return Ok(json!({
                "ok": false,
                "status": "not-ready",
                "method": probe.method,
                "path": probe.path,
                "target": probe.target,
                "note": probe.note,
                "error": error,
                "ms": started.elapsed().as_millis() as u64
            }))
        }
    };

    let result = match (probe.target, probe.method) {
        ("local", "GET") => session.local_get(&path).await.map_err(|error| error.to_string()),
        ("pd", "GET") => session.pd_get(&path).await.map_err(|error| error.to_string()),
        ("pd", "POST") => {
            let body = if probe.id == "storefront" {
                Some(json!({}))
            } else {
                None
            };
            session
                .pd_post(&path, body.as_ref())
                .await
                .map_err(|error| error.to_string())
        }
        ("glz", "GET") => session.glz_get(&path).await.map_err(|error| error.to_string()),
        ("shared", "GET") => session
            .shared_get(&path)
            .await
            .map_err(|error| error.to_string()),
        ("auth", "GET") => remote_request(&session, Method::GET, &path, None).await,
        ("auth", "POST") => {
            remote_request(&session, Method::POST, &path, Some(&json!({}))).await
        }
        ("clientconfig", "GET") => {
            remote_request(&session, Method::GET, &path, None).await
        }
        _ => Err("This transport is not implemented.".to_string()),
    };

    let ms = started.elapsed().as_millis() as u64;
    match result {
        Ok(data) => {
            let data = if probe.id == "riot_client_config" {
                client_config_summary(&data)
            } else {
                data
            };
            Ok(json!({
                "ok": true,
                "status": "ok",
                "method": probe.method,
                "path": path,
                "target": probe.target,
                "note": probe.note,
                "ms": ms,
                "data": redact(data)
            }))
        }
        Err(error) => Ok(json!({
            "ok": false,
            "status": "error",
            "method": probe.method,
            "path": path,
            "target": probe.target,
            "note": probe.note,
            "error": error,
            "ms": ms
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_covers_every_category() {
        for category in [
            "XMPP",
            "PVP",
            "Party",
            "Store",
            "Pre-Game",
            "Current Game",
            "Contracts",
            "Local",
            "Local Chat",
            "Authentication",
        ] {
            assert!(
                PROBES.iter().any(|probe| probe.category == category),
                "missing category {category}"
            );
        }
    }

    #[test]
    fn read_probes_use_http_methods() {
        for probe in PROBES.iter().filter(|probe| probe.status == "read") {
            assert!(
                ["GET", "POST"].contains(&probe.method),
                "{} uses unsupported method {}",
                probe.id,
                probe.method
            );
            assert!(!probe.path.is_empty(), "{} has no path", probe.id);
        }
    }

    #[test]
    fn redaction_hides_tokens_but_keeps_shape() {
        let value = redact(json!({
            "accessToken": "super-secret",
            "entitlements_token": "another-secret",
            "subject": "player-id",
            "nested": [{ "id_token": "id-secret" }]
        }));
        assert_eq!(value["accessToken"], "[redacted 12 chars]");
        assert_eq!(value["entitlements_token"], "[redacted 14 chars]");
        assert_eq!(value["subject"], "player-id");
        assert_eq!(value["nested"][0]["id_token"], "[redacted 9 chars]");
    }
}
