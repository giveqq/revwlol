use chrono::Utc;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::Mutex;

static DDRAGON: Lazy<Mutex<Option<DdragonCache>>> = Lazy::new(|| Mutex::new(None));

#[derive(Debug, Clone)]
struct Lockfile {
    port: String,
    password: String,
    protocol: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RevealPlayer {
    riot_id: Option<String>,
    game_name: Option<String>,
    tag_line: Option<String>,
    puuid: Option<String>,
    summoner_name: Option<String>,
    is_self: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LcuState {
    ok: bool,
    lockfile_found: bool,
    phase: String,
    message: String,
    current_summoner: Option<String>,
    champion_select_active: bool,
    revealed_players: Vec<RevealPlayer>,
    raw_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ChampionMini {
    name: String,
    key: String,
    games: u32,
    wins: u32,
    icon_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayerCard {
    riot_id: String,
    puuid: String,
    opgg_url: String,
    ugg_url: String,
    deeplol_url: String,
    mmradar_url: String,
    rank_text: String,
    main_role: String,
    profile_label: String,
    profile_tone: String,
    summary: String,
    confidence: String,
    metrics: HashMap<String, String>,
    tags: Vec<String>,
    champions: Vec<ChampionMini>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanResponse {
    generated_at: String,
    team_status: String,
    team_summary: String,
    dodge_label: String,
    eomm_pressure: String,
    players: Vec<PlayerCard>,
}

#[derive(Clone)]
struct DdragonCache {
    version: String,
    id_to_key: HashMap<i64, (String, String)>,
}

#[derive(Default, Clone)]
struct PlayerStats {
    games: u32,
    wins: u32,
    kills: u32,
    deaths: u32,
    assists: u32,
    kp_sum: f64,
    cs_pm_sum: f64,
    dmg_share_sum: f64,
    role_games: HashMap<String, u32>,
    champ_games: HashMap<i64, (u32, u32)>,
    recent_results: Vec<bool>,
    low_impact_wins: u32,
    good_losses: u32,
    death_heavy: u32,
}

fn candidate_lockfiles(manual: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if !manual.trim().is_empty() {
        let p = PathBuf::from(manual.trim());
        if p.is_dir() { paths.push(p.join("lockfile")); } else { paths.push(p); }
    }
    paths.push(PathBuf::from(r"C:\Riot Games\League of Legends\lockfile"));
    paths.push(PathBuf::from(r"C:\Program Files\Riot Games\League of Legends\lockfile"));
    paths.push(PathBuf::from(r"C:\Program Files (x86)\Riot Games\League of Legends\lockfile"));
    if let Ok(local) = env::var("LOCALAPPDATA") {
        paths.push(PathBuf::from(local).join(r"Riot Games\League of Legends\lockfile"));
    }
    paths
}

fn read_lockfile(manual: &str) -> Result<Lockfile, String> {
    for path in candidate_lockfiles(manual) {
        if path.exists() {
            let raw = fs::read_to_string(&path).map_err(|e| format!("Nie mogę odczytać lockfile: {e}"))?;
            let parts: Vec<&str> = raw.trim().split(':').collect();
            if parts.len() >= 5 {
                return Ok(Lockfile {
                    port: parts[2].to_string(),
                    password: parts[3].to_string(),
                    protocol: parts[4].to_string(),
                });
            }
        }
    }
    Err("Nie znaleziono lockfile klienta LoL".to_string())
}

async fn lcu_get(lock: &Lockfile, endpoint: &str) -> Result<Value, String> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}://127.0.0.1:{}{}", lock.protocol, lock.port, endpoint);
    let res = client
        .get(url)
        .basic_auth("riot", Some(&lock.password))
        .send()
        .await
        .map_err(|e| format!("LCU request failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("LCU {} zwróciło {}", endpoint, res.status()));
    }
    res.json::<Value>().await.map_err(|e| e.to_string())
}

fn extract_riot_id(v: &Value) -> RevealPlayer {
    let game_name = v.get("gameName").and_then(|x| x.as_str())
        .or_else(|| v.get("game_name").and_then(|x| x.as_str()))
        .or_else(|| v.get("name").and_then(|x| x.as_str()))
        .map(|s| s.to_string());
    let tag_line = v.get("gameTag").and_then(|x| x.as_str())
        .or_else(|| v.get("tagLine").and_then(|x| x.as_str()))
        .or_else(|| v.get("tag_line").and_then(|x| x.as_str()))
        .map(|s| s.trim_start_matches('#').to_string());
    let puuid = v.get("puuid").and_then(|x| x.as_str()).map(|s| s.to_string());
    let summoner_name = v.get("summonerName").and_then(|x| x.as_str())
        .or_else(|| v.get("summonerInternalName").and_then(|x| x.as_str()))
        .map(|s| s.to_string());
    let riot_id = match (&game_name, &tag_line) {
        (Some(g), Some(t)) if !g.is_empty() && !t.is_empty() => Some(format!("{}#{}", g, t)),
        _ => None,
    };
    RevealPlayer {
        riot_id,
        game_name,
        tag_line,
        puuid,
        summoner_name,
        is_self: v.get("isSelf").and_then(|x| x.as_bool()).unwrap_or(false),
    }
}

async fn reveal_like_players(lock: &Lockfile) -> Vec<RevealPlayer> {
    let mut out = Vec::new();
    if let Ok(champ) = lcu_get(lock, "/lol-champ-select/v1/session").await {
        if let Some(arr) = champ.get("myTeam").and_then(|x| x.as_array()) {
            for p in arr { out.push(extract_riot_id(p)); }
        }
    }
    if let Ok(conversations) = lcu_get(lock, "/lol-chat/v1/conversations").await {
        if let Some(arr) = conversations.as_array() {
            for conv in arr {
                let id = conv.get("id").and_then(|x| x.as_str()).unwrap_or("");
                let typ = conv.get("type").and_then(|x| x.as_str()).unwrap_or("").to_lowercase();
                let name = conv.get("name").and_then(|x| x.as_str()).unwrap_or("").to_lowercase();
                if id.is_empty() { continue; }
                if typ.contains("champ") || name.contains("champ") || name.contains("select") || name.contains("lobby") {
                    let endpoint = format!("/lol-chat/v1/conversations/{}/participants", id);
                    if let Ok(parts) = lcu_get(lock, &endpoint).await {
                        if let Some(pa) = parts.as_array() {
                            for p in pa { out.push(extract_riot_id(p)); }
                        }
                    }
                }
            }
        }
    }
    let mut seen = HashMap::new();
    out.into_iter().filter(|p| {
        let key = p.riot_id.clone().or_else(|| p.puuid.clone()).or_else(|| p.summoner_name.clone()).unwrap_or_default();
        if key.is_empty() { return false; }
        if seen.contains_key(&key) { false } else { seen.insert(key, true); true }
    }).collect()
}

#[tauri::command]
async fn get_lcu_state(manual_lockfile_path: String) -> LcuState {
    let lock = match read_lockfile(&manual_lockfile_path) {
        Ok(x) => x,
        Err(e) => return LcuState { ok: false, lockfile_found: false, phase: "Offline".into(), message: e, current_summoner: None, champion_select_active: false, revealed_players: vec![], raw_count: 0 },
    };
    let gameflow = lcu_get(&lock, "/lol-gameflow/v1/session").await.ok();
    let phase = gameflow.as_ref()
        .and_then(|v| v.get("phase"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();
    let current_summoner = lcu_get(&lock, "/lol-summoner/v1/current-summoner").await.ok()
        .and_then(|v| v.get("displayName").and_then(|x| x.as_str()).map(|s| s.to_string()));
    let champion_select_active = phase == "ChampSelect" || lcu_get(&lock, "/lol-champ-select/v1/session").await.is_ok();
    let players = if champion_select_active { reveal_like_players(&lock).await } else { vec![] };
    let msg = if champion_select_active {
        format!("Champ select aktywny · wykryto {} graczy", players.len())
    } else if phase == "InProgress" {
        "Jesteś w grze / Practice Tool — draft nieaktywny".to_string()
    } else {
        format!("Klient wykryty · faza: {phase}")
    };
    LcuState { ok: true, lockfile_found: true, phase, message: msg, current_summoner, champion_select_active, raw_count: players.len(), revealed_players: players }
}

async fn riot_get(api_key: &str, url: &str) -> Result<Value, String> {
    let client = Client::builder().timeout(Duration::from_secs(8)).build().map_err(|e| e.to_string())?;
    let res = client.get(url).header("X-Riot-Token", api_key).send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() { return Err(format!("Riot API {} -> {}", url, res.status())); }
    res.json::<Value>().await.map_err(|e| e.to_string())
}

async fn ddragon() -> Result<DdragonCache, String> {
    let mut guard = DDRAGON.lock().await;
    if let Some(cache) = guard.clone() { return Ok(cache); }
    let client = Client::builder().timeout(Duration::from_secs(8)).build().map_err(|e| e.to_string())?;
    let versions: Vec<String> = client.get("https://ddragon.leagueoflegends.com/api/versions.json").send().await.map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
    let version = versions.first().cloned().unwrap_or_else(|| "15.1.1".into());
    let url = format!("https://ddragon.leagueoflegends.com/cdn/{}/data/en_US/champion.json", version);
    let json: Value = client.get(url).send().await.map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
    let mut id_to_key = HashMap::new();
    if let Some(data) = json.get("data").and_then(|x| x.as_object()) {
        for (key, champ) in data {
            if let Some(id_str) = champ.get("key").and_then(|x| x.as_str()) {
                if let Ok(id) = id_str.parse::<i64>() {
                    let name = champ.get("name").and_then(|x| x.as_str()).unwrap_or(key).to_string();
                    id_to_key.insert(id, (key.clone(), name));
                }
            }
        }
    }
    let cache = DdragonCache { version, id_to_key };
    *guard = Some(cache.clone());
    Ok(cache)
}

fn split_riot_id(input: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = input.split('#').collect();
    if parts.len() != 2 || parts[0].trim().is_empty() || parts[1].trim().is_empty() {
        return Err(format!("Niepoprawny Riot ID: {input}"));
    }
    Ok((parts[0].trim().to_string(), parts[1].trim().trim_start_matches('#').to_string()))
}

fn role_label(role: &str) -> String {
    match role {
        "TOP" => "Top".into(), "JUNGLE" => "Jungle".into(), "MIDDLE" => "Mid".into(), "BOTTOM" => "ADC".into(), "UTILITY" => "Support".into(), _ => "Mixed".into(),
    }
}

fn opgg_region(platform: &str) -> String {
    match platform { "euw1" => "euw", "eun1" => "eune", "na1" => "na", "kr" => "kr", "br1" => "br", "jp1" => "jp", "tr1" => "tr", "ru" => "ru", "oc1" => "oce", "la1" => "lan", "la2" => "las", _ => "euw" }.into()
}

async fn scan_one(api_key: &str, riot_id: &str, platform: &str, region: &str, force_refresh: bool) -> Result<PlayerCard, String> {
    let (game_name, tag) = split_riot_id(riot_id)?;
    let account_url = format!("https://{}.api.riotgames.com/riot/account/v1/accounts/by-riot-id/{}/{}", region, urlencoding::encode(&game_name), urlencoding::encode(&tag));
    let account = riot_get(api_key, &account_url).await?;
    let puuid = account.get("puuid").and_then(|x| x.as_str()).ok_or("Brak PUUID")?.to_string();
    let clean_riot_id = format!("{}#{}", account.get("gameName").and_then(|x| x.as_str()).unwrap_or(&game_name), account.get("tagLine").and_then(|x| x.as_str()).unwrap_or(&tag));

    let summoner_url = format!("https://{}.api.riotgames.com/lol/summoner/v4/summoners/by-puuid/{}", platform, puuid);
    let summoner = riot_get(api_key, &summoner_url).await.unwrap_or(Value::Null);
    let encrypted_id = summoner.get("id").and_then(|x| x.as_str()).unwrap_or("");
    let mut rank_text = "Unranked".to_string();
    if !encrypted_id.is_empty() {
        let league_url = format!("https://{}.api.riotgames.com/lol/league/v4/entries/by-summoner/{}", platform, encrypted_id);
        if let Ok(league) = riot_get(api_key, &league_url).await {
            if let Some(arr) = league.as_array() {
                if let Some(solo) = arr.iter().find(|e| e.get("queueType").and_then(|x| x.as_str()) == Some("RANKED_SOLO_5x5")) {
                    let tier = solo.get("tier").and_then(|x| x.as_str()).unwrap_or("");
                    let rank = solo.get("rank").and_then(|x| x.as_str()).unwrap_or("");
                    let lp = solo.get("leaguePoints").and_then(|x| x.as_i64()).unwrap_or(0);
                    let wins = solo.get("wins").and_then(|x| x.as_i64()).unwrap_or(0);
                    let losses = solo.get("losses").and_then(|x| x.as_i64()).unwrap_or(0);
                    let wr = if wins + losses > 0 { wins as f64 / (wins + losses) as f64 * 100.0 } else { 0.0 };
                    rank_text = format!("{} {} · {} LP · {:.0}% WR", tier, rank, lp, wr);
                }
            }
        }
    }

    let count = if force_refresh { 30 } else { 12 };
    let ids_url = format!("https://{}.api.riotgames.com/lol/match/v5/matches/by-puuid/{}/ids?queue=420&start=0&count={}", region, puuid, count);
    let match_ids = riot_get(api_key, &ids_url).await?.as_array().cloned().unwrap_or_default();
    let mut stats = PlayerStats::default();

    for mid in match_ids.iter().filter_map(|v| v.as_str()) {
        let url = format!("https://{}.api.riotgames.com/lol/match/v5/matches/{}", region, mid);
        let m = match riot_get(api_key, &url).await { Ok(v) => v, Err(_) => continue };
        let participants = match m.pointer("/info/participants").and_then(|x| x.as_array()) { Some(x) => x, None => continue };
        let me = match participants.iter().find(|p| p.get("puuid").and_then(|x| x.as_str()) == Some(&puuid)) { Some(x) => x, None => continue };
        let team_id = me.get("teamId").and_then(|x| x.as_i64()).unwrap_or(0);
        let team_kills: i64 = participants.iter().filter(|p| p.get("teamId").and_then(|x| x.as_i64()).unwrap_or(0) == team_id).map(|p| p.get("kills").and_then(|x| x.as_i64()).unwrap_or(0)).sum();
        let team_dmg: i64 = participants.iter().filter(|p| p.get("teamId").and_then(|x| x.as_i64()).unwrap_or(0) == team_id).map(|p| p.get("totalDamageDealtToChampions").and_then(|x| x.as_i64()).unwrap_or(0)).sum();
        let game_duration = m.pointer("/info/gameDuration").and_then(|x| x.as_f64()).unwrap_or(1800.0).max(60.0);
        let kills = me.get("kills").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let deaths = me.get("deaths").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let assists = me.get("assists").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let win = me.get("win").and_then(|x| x.as_bool()).unwrap_or(false);
        let champ_id = me.get("championId").and_then(|x| x.as_i64()).unwrap_or(0);
        let pos = me.get("teamPosition").and_then(|x| x.as_str()).unwrap_or("UNKNOWN").to_string();
        let cs = me.get("totalMinionsKilled").and_then(|x| x.as_f64()).unwrap_or(0.0) + me.get("neutralMinionsKilled").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let dmg = me.get("totalDamageDealtToChampions").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let kp = if team_kills > 0 { (kills + assists) as f64 / team_kills as f64 } else { 0.0 };
        let dmg_share = if team_dmg > 0 { dmg / team_dmg as f64 } else { 0.0 };

        stats.games += 1;
        if win { stats.wins += 1; }
        stats.kills += kills; stats.deaths += deaths; stats.assists += assists;
        stats.kp_sum += kp; stats.dmg_share_sum += dmg_share; stats.cs_pm_sum += cs / (game_duration / 60.0);
        *stats.role_games.entry(pos).or_insert(0) += 1;
        let e = stats.champ_games.entry(champ_id).or_insert((0,0)); e.0 += 1; if win { e.1 += 1; }
        stats.recent_results.push(win);
        if win && kp < 0.35 && dmg_share < 0.18 { stats.low_impact_wins += 1; }
        if !win && kp > 0.55 && deaths <= 5 && dmg_share > 0.22 { stats.good_losses += 1; }
        if deaths >= 8 { stats.death_heavy += 1; }
        tokio::time::sleep(Duration::from_millis(70)).await;
    }

    let dd = ddragon().await?;
    let main_role = stats.role_games.iter().max_by_key(|(_, v)| *v).map(|(k,_)| role_label(k)).unwrap_or("Unknown".into());
    let wr = if stats.games > 0 { stats.wins as f64 / stats.games as f64 * 100.0 } else { 0.0 };
    let kda = if stats.deaths > 0 { (stats.kills + stats.assists) as f64 / stats.deaths as f64 } else { (stats.kills + stats.assists) as f64 };
    let kp = if stats.games > 0 { stats.kp_sum / stats.games as f64 * 100.0 } else { 0.0 };
    let cs = if stats.games > 0 { stats.cs_pm_sum / stats.games as f64 } else { 0.0 };
    let role_consistency = stats.role_games.values().max().cloned().unwrap_or(0) as f64 / stats.games.max(1) as f64;

    let mut champ_vec: Vec<(i64,u32,u32)> = stats.champ_games.iter().map(|(id,(g,w))| (*id,*g,*w)).collect();
    champ_vec.sort_by_key(|(_,g,_)| std::cmp::Reverse(*g));
    let champions = champ_vec.iter().take(3).map(|(id,g,w)| {
        let (key, name) = dd.id_to_key.get(id).cloned().unwrap_or(("Unknown".into(), "Unknown".into()));
        ChampionMini { name, key: key.clone(), games: *g, wins: *w, icon_url: format!("https://ddragon.leagueoflegends.com/cdn/{}/img/champion/{}.png", dd.version, key) }
    }).collect::<Vec<_>>();

    let mut tags = Vec::new();
    if role_consistency >= 0.70 { tags.push("main role".into()); } else if role_consistency < 0.45 { tags.push("mixed roles".into()); }
    if wr >= 58.0 { tags.push("strong recent WR".into()); }
    if wr < 42.0 && stats.games >= 8 { tags.push("spadkowa forma".into()); }
    if stats.low_impact_wins >= 3 { tags.push("często carrowany".into()); }
    if stats.good_losses >= 2 { tags.push("dobre staty w porażkach".into()); }
    if stats.death_heavy >= 4 { tags.push("death-heavy".into()); }
    if champions.first().map(|c| c.games >= 4).unwrap_or(false) { tags.push("comfort champ".into()); }

    let (label, tone, summary) = if stats.games < 5 {
        ("Mało danych".into(), "neutral".into(), "Profil ma mało ostatnich gier SoloQ, więc ocena jest ostrożna.".into())
    } else if wr >= 58.0 && kda >= 2.7 && role_consistency >= 0.6 {
        ("Winnable".into(), "positive".into(), "Wygląda stabilnie: dobra recent forma, sensowny impact i najczęściej gra swoją rolę.".into())
    } else if stats.low_impact_wins >= 3 && wr >= 50.0 {
        ("Carried wins".into(), "neutral".into(), "Wyniki wyglądają ok, ale część wygranych ma niski indywidualny impact.".into())
    } else if wr < 42.0 || stats.death_heavy >= 4 {
        ("Risky".into(), "negative".into(), "Ryzykowny profil: słabsza recent forma lub dużo gier z wysoką liczbą śmierci.".into())
    } else {
        ("Playable".into(), "neutral".into(), "Profil wygląda grywalnie, bez dużych czerwonych flag w ostatnich SoloQ.".into())
    };

    let mut metrics = HashMap::new();
    metrics.insert("Recent".into(), format!("{:.0}%", wr));
    metrics.insert("KDA".into(), format!("{:.2}", kda));
    metrics.insert("KP".into(), format!("{:.0}%", kp));
    metrics.insert("CS/min".into(), format!("{:.1}", cs));
    metrics.insert("Games".into(), format!("{}", stats.games));
    metrics.insert("Role".into(), format!("{:.0}%", role_consistency*100.0));

    let region_slug = opgg_region(platform);
    let encoded = clean_riot_id.replace('#', "-");
    Ok(PlayerCard {
        riot_id: clean_riot_id.clone(), puuid,
        opgg_url: format!("https://op.gg/lol/summoners/{}/{}", region_slug, urlencoding::encode(&encoded)),
        ugg_url: format!("https://u.gg/lol/profile/{}/{}//overview", platform, urlencoding::encode(&encoded)),
        deeplol_url: format!("https://www.deeplol.gg/summoner/{}/{}", region_slug, urlencoding::encode(&encoded)),
        mmradar_url: format!("https://mmradar.gg/summoner/{}?search=1", urlencoding::encode(&encoded)),
        rank_text, main_role, profile_label: label, profile_tone: tone, summary,
        confidence: if stats.games >= 15 { "high".into() } else if stats.games >= 8 { "medium".into() } else { "low".into() },
        metrics, tags, champions,
    })
}

#[tauri::command]
async fn scan_riot_ids(api_key: String, riot_ids: Vec<String>, platform: String, region: String, my_riot_id: String, hide_own_profile: bool, force_refresh: bool) -> Result<ScanResponse, String> {
    if api_key.trim().is_empty() { return Err("Brak Riot API key".into()); }
    let mut cards = Vec::new();
    for riot_id in riot_ids.iter().take(5) {
        match scan_one(&api_key, riot_id, &platform, &region, force_refresh).await {
            Ok(card) => cards.push(card),
            Err(e) => cards.push(PlayerCard {
                riot_id: riot_id.clone(), puuid: "".into(), opgg_url: "".into(), ugg_url: "".into(), deeplol_url: "".into(), mmradar_url: "".into(), rank_text: "error".into(), main_role: "Unknown".into(), profile_label: "Błąd".into(), profile_tone: "negative".into(), summary: e, confidence: "low".into(), metrics: HashMap::new(), tags: vec!["scan error".into()], champions: vec![]
            }),
        }
        tokio::time::sleep(Duration::from_millis(140)).await;
    }
    let visible: Vec<&PlayerCard> = cards.iter().filter(|p| !(hide_own_profile && !my_riot_id.is_empty() && p.riot_id.eq_ignore_ascii_case(&my_riot_id))).collect();
    let risky = visible.iter().filter(|p| p.profile_tone == "negative").count();
    let positive = visible.iter().filter(|p| p.profile_tone == "positive").count();
    let team_status = if risky >= 2 { "Ryzykowne lobby" } else if positive >= 2 && risky == 0 { "Winnable lobby" } else { "Playable lobby" }.to_string();
    let dodge_label = if risky >= 2 { "Dodge consider" } else if risky == 1 { "Uważaj" } else { "Grywalne" }.to_string();
    let eomm_pressure = if risky >= 2 { "high-pressure pattern" } else if risky == 1 { "medium" } else { "low" }.to_string();
    let team_summary = if risky >= 2 {
        "Kilka profili ma słabszą recent formę albo wysokie ryzyko. To nie dowód na losers queue, ale lobby wygląda presyjnie.".to_string()
    } else if positive >= 2 {
        "Team wygląda solidnie: kilka profili ma dobrą recent formę i sensowne role/champion comfort.".to_string()
    } else {
        "Lobby wygląda neutralnie. Brak dużych czerwonych flag, ale też brak mocnego carry patternu.".to_string()
    };
    Ok(ScanResponse { generated_at: Utc::now().to_rfc3339(), team_status, team_summary, dodge_label, eomm_pressure, players: cards })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_lcu_state, scan_riot_ids])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
