use serde::Serialize;

pub const PARTY_COLORS: [&str; 8] = [
    "#E34343", "#D843E3", "#4346E3", "#43E3D0", "#5EE343", "#E2ED39", "#D452CF", "#E38F43",
];

#[derive(Clone, Copy)]
struct AgentDef {
    name: &'static str,
    uuid: &'static str,
    role: &'static str,
    color: &'static str,
}

const AGENTS: &[AgentDef] = &[
    AgentDef { name: "Jett", uuid: "add6443a-41bd-e414-f6ad-e58d267f4e95", role: "Duelist", color: "#9ADEFF" },
    AgentDef { name: "Phoenix", uuid: "eb93336a-449b-9c1b-0a54-a891f7921d69", role: "Duelist", color: "#FE8266" },
    AgentDef { name: "Reyna", uuid: "a3bfb853-43b2-7238-a4f1-ad90e9e46bcc", role: "Duelist", color: "#B565B5" },
    AgentDef { name: "Raze", uuid: "f94c3b30-42be-e959-889c-5aa313dba261", role: "Duelist", color: "#FFA400" },
    AgentDef { name: "Yoru", uuid: "7f94d92c-4234-0a36-9646-3a87eb8b5c89", role: "Duelist", color: "#2846C8" },
    AgentDef { name: "Neon", uuid: "bb2a4828-46eb-8cd1-e765-15848195d751", role: "Duelist", color: "#00CFFF" },
    AgentDef { name: "Iso", uuid: "0e38b510-41a8-5780-5e8f-568b2a4f2d6c", role: "Duelist", color: "#574AC2" },
    AgentDef { name: "Waylay", uuid: "df1cb487-4902-002e-5c17-d28e83e78588", role: "Duelist", color: "#82C3EB" },
    AgentDef { name: "Hush", uuid: "ddd5a15c-4fbe-42fa-3789-11a55ca5ee04", role: "Duelist", color: "#F47AC7" },
    AgentDef { name: "Sova", uuid: "320b2a48-4d9b-a075-30f1-1f93a9b638fa", role: "Initiator", color: "#3BA0E5" },
    AgentDef { name: "Breach", uuid: "5f8d3a7f-467b-97f3-062c-13acf203c006", role: "Initiator", color: "#C76B3B" },
    AgentDef { name: "Skye", uuid: "6f2a04ca-43e0-be17-7f36-b3908627744d", role: "Initiator", color: "#C0E69E" },
    AgentDef { name: "KAY/O", uuid: "601dbbe7-43ce-be57-2a40-4abd24953621", role: "Initiator", color: "#85929C" },
    AgentDef { name: "Fade", uuid: "dade69b4-4f5a-8528-247b-219e5a1facd6", role: "Initiator", color: "#5C5C5E" },
    AgentDef { name: "Gekko", uuid: "e370fa57-4757-3604-3648-499e1f642d3f", role: "Initiator", color: "#A8E65E" },
    AgentDef { name: "Tejo", uuid: "b444168c-4e35-8076-db47-ef9bf368f384", role: "Initiator", color: "#FFB761" },
    AgentDef { name: "Brimstone", uuid: "9f0d8ba9-4140-b941-57d3-a7ad57c6b417", role: "Controller", color: "#D1691F" },
    AgentDef { name: "Omen", uuid: "8e253930-4c05-31dd-1b6c-968525494517", role: "Controller", color: "#47508F" },
    AgentDef { name: "Viper", uuid: "707eab51-4836-f488-046a-cda6bf494859", role: "Controller", color: "#38C659" },
    AgentDef { name: "Astra", uuid: "41fb69c1-4189-7b37-f117-bcaf1e96f1bf", role: "Controller", color: "#712AE8" },
    AgentDef { name: "Harbor", uuid: "95b78ed7-4637-86d9-7e41-71ba8c293152", role: "Controller", color: "#008080" },
    AgentDef { name: "Clove", uuid: "1dbf2edd-4729-0984-3115-daa5eed44993", role: "Controller", color: "#F28FD0" },
    AgentDef { name: "Miks", uuid: "7c8a4701-4de6-9355-b254-e09bc2a34b72", role: "Controller", color: "#6B4BB0" },
    AgentDef { name: "Sage", uuid: "569fdd95-4d10-43ab-ca70-79becc718b46", role: "Sentinel", color: "#26C8AF" },
    AgentDef { name: "Cypher", uuid: "117ed9e3-49f3-6512-3ccf-0cada7e3823b", role: "Sentinel", color: "#E6D9C5" },
    AgentDef { name: "Killjoy", uuid: "1e58de9c-4950-5125-93e9-a0aee9f98746", role: "Sentinel", color: "#FFD91F" },
    AgentDef { name: "Chamber", uuid: "22697a3d-45bf-8dd7-4fec-84a9e28c69d7", role: "Sentinel", color: "#B89A46" },
    AgentDef { name: "Deadlock", uuid: "cc8b64c8-4b25-4ff9-6e7f-37b4da43d235", role: "Sentinel", color: "#6677B0" },
    AgentDef { name: "Vyse", uuid: "efba5359-4016-a1e5-7626-b1ae76895940", role: "Sentinel", color: "#656B8B" },
    AgentDef { name: "Veto", uuid: "92eeef5d-43b5-1d4a-8d03-b3927a09034b", role: "Sentinel", color: "#2E8A96" },
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub name: &'static str,
    pub uuid: &'static str,
    pub role: &'static str,
    pub color: &'static str,
    pub portrait: String,
    pub full_portrait: String,
}

pub fn agents() -> Vec<Agent> {
    AGENTS
        .iter()
        .map(|a| Agent {
            name: a.name,
            uuid: a.uuid,
            role: a.role,
            color: a.color,
            portrait: format!(
                "https://media.valorant-api.com/agents/{}/displayicon.png",
                a.uuid
            ),
            full_portrait: format!(
                "https://media.valorant-api.com/agents/{}/fullportrait.png",
                a.uuid
            ),
        })
        .collect()
}

pub fn agents_json() -> serde_json::Value {
    serde_json::json!({ "agents": agents(), "count": AGENTS.len() })
}

pub fn resolve_agent(identifier: &str) -> Option<Agent> {
    let needle = identifier.trim().to_lowercase();
    agents().into_iter().find(|a| {
        a.name.to_lowercase() == needle || a.uuid.eq_ignore_ascii_case(&needle)
    })
}

pub fn agent_name_by_uuid(uuid: &str) -> Option<&'static str> {
    AGENTS
        .iter()
        .find(|a| a.uuid.eq_ignore_ascii_case(uuid))
        .map(|a| a.name)
}

pub fn agent_color(name: &str) -> &'static str {
    AGENTS
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(name))
        .map(|a| a.color)
        .unwrap_or("#8B978F")
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Rank {
    pub tier: i32,
    pub name: &'static str,
    pub group: &'static str,
    pub color: &'static str,
}

const RANK_GROUPS: &[(&str, &[&str], &str)] = &[
    ("Unranked", &["Unranked"], "#4A4A4A"),
    ("Iron", &["Iron 1", "Iron 2", "Iron 3"], "#5A5751"),
    ("Bronze", &["Bronze 1", "Bronze 2", "Bronze 3"], "#BB8F5A"),
    ("Silver", &["Silver 1", "Silver 2", "Silver 3"], "#AEB2B2"),
    ("Gold", &["Gold 1", "Gold 2", "Gold 3"], "#C5BA3F"),
    ("Platinum", &["Platinum 1", "Platinum 2", "Platinum 3"], "#18A7B9"),
    ("Diamond", &["Diamond 1", "Diamond 2", "Diamond 3"], "#D864C7"),
    ("Ascendant", &["Ascendant 1", "Ascendant 2", "Ascendant 3"], "#189452"),
    ("Immortal", &["Immortal 1", "Immortal 2", "Immortal 3"], "#DD4444"),
    ("Radiant", &["Radiant"], "#FFFDCD"),
];

pub fn rank_from_tier(tier: i32) -> Rank {
    let clamped = tier.clamp(0, 25);
    let mut index = 0usize;
    for (group, names, color) in RANK_GROUPS {
        for name in *names {
            if index == clamped as usize {
                return Rank {
                    tier: index as i32,
                    name,
                    group,
                    color,
                };
            }
            index += 1;
        }
    }
    Rank {
        tier: 0,
        name: "Unranked",
        group: "Unranked",
        color: "#4A4A4A",
    }
}

pub fn rank_icon(tier: i32) -> String {
    format!(
        "https://media.valorant-api.com/competitivetiers/03621f52-342b-cf4e-4f86-9350a49c6d04/{}/smallicon.png",
        tier.clamp(0, 25)
    )
}

pub fn map_name_from_id(map_id: &str) -> String {
    if map_id.is_empty() {
        return "Unknown".into();
    }
    let leaf = map_id.rsplit('/').next().unwrap_or(map_id);
    match leaf {
        "Triad" => "Haven",
        "Duality" => "Bind",
        "Bonsai" => "Split",
        "Port" => "Icebox",
        "Foxtrot" => "Breeze",
        "Canyon" => "Fracture",
        "Pitt" => "Pearl",
        "Jam" => "Lotus",
        "Juliett" => "Sunset",
        "Infinity" => "Abyss",
        "Rook" => "Corrode",
        "Plummet" => "Summit",
        "HURM_Alley" => "District",
        "HURM_Bowl" => "Kasbah",
        "HURM_Helix" => "Drift",
        "HURM_HighTide" => "Glitch",
        "HURM_Yard" => "Piazza",
        other => other,
    }
    .to_string()
}

pub fn mode_label(queue: &str) -> &'static str {
    match queue.to_lowercase().as_str() {
        "competitive" => "Competitive",
        "unrated" => "Unrated",
        "swiftplay" => "Swiftplay",
        "spikerush" => "Spike Rush",
        "deathmatch" => "Deathmatch",
        "ggteam" => "Escalation",
        "hurm" => "Team Deathmatch",
        "fortcollins" => "Retake",
        "skirmish2v2" => "Skirmish 2v2",
        "custom" | "" => "Custom",
        _ => "Custom",
    }
}
