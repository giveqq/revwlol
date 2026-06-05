<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { open } from '@tauri-apps/api/shell';
  import { onMount } from 'svelte';

  type Settings = {
    apiKey: string;
    myRiotId: string;
    platform: string;
    region: string;
    lockfilePath: string;
    hideOwnProfile: boolean;
    autoAnalyzeReveal: boolean;
  };

  type LcuState = {
    ok: boolean;
    lockfileFound: boolean;
    phase: string;
    message: string;
    currentSummoner?: string | null;
    championSelectActive: boolean;
    revealedPlayers: RevealPlayer[];
    rawCount: number;
  };

  type RevealPlayer = {
    riotId?: string | null;
    gameName?: string | null;
    tagLine?: string | null;
    puuid?: string | null;
    summonerName?: string | null;
    isSelf?: boolean;
  };

  type ScanResponse = {
    generatedAt: string;
    teamStatus: string;
    teamSummary: string;
    dodgeLabel: string;
    eommPressure: string;
    players: PlayerCard[];
  };

  type PlayerCard = {
    riotId: string;
    puuid: string;
    opggUrl: string;
    uggUrl: string;
    deeplolUrl: string;
    mmradarUrl: string;
    rankText: string;
    mainRole: string;
    profileLabel: string;
    profileTone: string;
    summary: string;
    confidence: string;
    metrics: Record<string, string>;
    tags: string[];
    champions: ChampionMini[];
  };

  type ChampionMini = {
    name: string;
    key: string;
    games: number;
    wins: number;
    iconUrl: string;
  };

  const defaultSettings: Settings = {
    apiKey: '',
    myRiotId: '',
    platform: 'euw1',
    region: 'europe',
    lockfilePath: '',
    hideOwnProfile: true,
    autoAnalyzeReveal: false,
  };

  let settings: Settings = { ...defaultSettings };
  let input = '';
  let forceRefresh = false;
  let showSettings = false;
  let loading = false;
  let error = '';
  let results: ScanResponse | null = null;
  let lcu: LcuState | null = null;
  let lastAutoKey = '';

  const platformOptions = [
    ['euw1', 'EUW'], ['eun1', 'EUNE'], ['na1', 'NA'], ['kr', 'KR'], ['br1', 'BR'], ['la1', 'LAN'], ['la2', 'LAS'], ['oc1', 'OCE'], ['tr1', 'TR'], ['ru', 'RU'], ['jp1', 'JP']
  ];

  function loadSettings() {
    const raw = localStorage.getItem('giverss-desktop-settings');
    if (raw) settings = { ...defaultSettings, ...JSON.parse(raw) };
  }

  function saveSettings() {
    localStorage.setItem('giverss-desktop-settings', JSON.stringify(settings));
    showSettings = false;
  }

  function parseRiotIds(text: string): string[] {
    const raw = text.trim();
    if (!raw) return [];
    const out: string[] = [];
    try {
      const url = new URL(raw);
      const summoners = url.searchParams.get('summoners');
      if (summoners) {
        for (const part of summoners.split(',')) {
          const decoded = decodeURIComponent(part).trim();
          if (decoded.includes('#')) out.push(decoded);
        }
      }
      const path = decodeURIComponent(url.pathname);
      const chunks = path.split('/').filter(Boolean);
      const last = chunks[chunks.length - 1] || '';
      if (last && !summoners) {
        const hyphen = last.lastIndexOf('-');
        if (hyphen > 0) out.push(`${last.slice(0, hyphen)}#${last.slice(hyphen + 1)}`);
      }
    } catch {
      for (const line of raw.split(/[\n,;]/)) {
        const cleaned = line.trim();
        if (cleaned.includes('#')) out.push(cleaned);
      }
    }
    return [...new Set(out.map(x => x.replace(/\s+#/g, '#').trim()).filter(Boolean))];
  }

  function revealedIds(): string[] {
    if (!lcu?.revealedPlayers) return [];
    return [...new Set(lcu.revealedPlayers
      .map(p => p.riotId || (p.gameName && p.tagLine ? `${p.gameName}#${p.tagLine}` : ''))
      .filter(Boolean) as string[])];
  }

  async function runScan(ids?: string[], force = false) {
    const riotIds = ids || parseRiotIds(input);
    if (!settings.apiKey.trim()) {
      error = 'Wpisz Riot API key w ustawieniach.';
      return;
    }
    if (!riotIds.length) {
      error = 'Wklej OP.GG multisearch, profil gracza albo Riot ID Name#TAG.';
      return;
    }
    loading = true;
    error = '';
    try {
      results = await invoke<ScanResponse>('scan_riot_ids', {
        apiKey: settings.apiKey.trim(),
        riotIds,
        platform: settings.platform,
        region: settings.region,
        myRiotId: settings.myRiotId.trim(),
        hideOwnProfile: settings.hideOwnProfile,
        forceRefresh: force || forceRefresh,
      });
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function pollLcu() {
    try {
      lcu = await invoke<LcuState>('get_lcu_state', { manualLockfilePath: settings.lockfilePath.trim() });
      const ids = revealedIds();
      const key = ids.join('|');
      if (settings.autoAnalyzeReveal && ids.length && settings.apiKey.trim() && key !== lastAutoKey) {
        lastAutoKey = key;
        await runScan(ids, false);
      }
    } catch (e) {
      lcu = { ok: false, lockfileFound: false, phase: 'Unknown', message: String(e), championSelectActive: false, revealedPlayers: [], rawCount: 0 };
    }
  }

  async function scanRevealNow() {
    const ids = revealedIds();
    if (!ids.length) {
      error = 'Nie wykryłem teammate\'ów z klienta. Wejdź w champ select albo wklej multisearch ręcznie.';
      return;
    }
    input = ids.join('\n');
    await runScan(ids, false);
  }

  function isOwn(p: PlayerCard) {
    return settings.myRiotId && p.riotId.toLowerCase() === settings.myRiotId.toLowerCase();
  }

  async function openUrl(url: string) { await open(url); }

  onMount(() => {
    loadSettings();
    pollLcu();
    const id = window.setInterval(pollLcu, 2500);
    return () => window.clearInterval(id);
  });
</script>

<div class="bg"></div>
<div class="rain" aria-hidden="true">
  {#each Array(120) as _, i}
    <i style={`--i:${i}; --x:${(i * 37) % 100}; --d:${(i * 13) % 30}; --s:${0.6 + ((i * 7) % 18) / 20}`}></i>
  {/each}
</div>

<main class="shell">
  <section class="hero">
    <div class="logo" aria-label="GIVERSS">GIVERSS</div>
    <div class="lcu-pill" class:ok={lcu?.ok} class:champ={lcu?.championSelectActive}>
      <span></span>{lcu?.message || 'Szukam klienta LoL...'}
    </div>

    <div class="search-row">
      <textarea bind:value={input} placeholder="Wklej OP.GG multisearch, profil OP.GG/U.GG albo Riot ID Name#TAG"></textarea>
      <div class="actions">
        <button class="primary" disabled={loading} on:click={() => runScan()}>{loading ? 'Skanuję…' : 'Szukaj'}</button>
        <button title="Wymuś świeże dane" disabled={loading} on:click={() => runScan(undefined, true)}>↻</button>
        <button class="ghost" on:click={() => showSettings = !showSettings}>⚙</button>
      </div>
    </div>

    {#if showSettings}
      <div class="settings-panel">
        <label>Riot API key<input type="password" bind:value={settings.apiKey} placeholder="RGAPI-..." /></label>
        <label>Twój Riot ID<input bind:value={settings.myRiotId} placeholder="ＯＢＳＥＳＳＩＯＮ#GIVE" /></label>
        <div class="settings-grid">
          <label>Platforma<select bind:value={settings.platform}>{#each platformOptions as [value, label]}<option value={value}>{label}</option>{/each}</select></label>
          <label>Routing<select bind:value={settings.region}><option value="europe">Europe</option><option value="americas">Americas</option><option value="asia">Asia</option><option value="sea">[...]</option></select></label>
        </div>
        <label>Lockfile opcjonalnie<input bind:value={settings.lockfilePath} placeholder="C:\Riot Games\League of Legends\lockfile" /></label>
        <label class="check"><input type="checkbox" bind:checked={settings.hideOwnProfile} /> Ukryj mój profil w wynikach</label>
        <label class="check"><input type="checkbox" bind:checked={settings.autoAnalyzeReveal} /> Automatycznie analizuj wykryte lobby</label>
        <button class="primary small" on:click={saveSettings}>Zapisz</button>
      </div>
    {/if}

    <div class="reveal-box">
      <div>
        <b>Reveal-like status:</b> {lcu?.phase || 'Unknown'} · wykryto {revealedIds().length} Riot ID
      </div>
      <button class="mini" on:click={scanRevealNow}>Skanuj wykryte lobby</button>
    </div>

    {#if error}<div class="error">{error}</div>{/if}
  </section>

  {#if results}
    <section class="summary-card slide-in">
      <div>
        <span class="eyebrow">Lobby status</span>
        <h2>{results.teamStatus}</h2>
        <p>{results.teamSummary}</p>
      </div>
      <div class="verdict">
        <strong>{results.dodgeLabel}</strong>
        <span>EOMM-like: {results.eommPressure}</span>
      </div>
    </section>

    <section class="cards slide-in delay">
      {#each results.players as p}
        {#if !(settings.hideOwnProfile && isOwn(p))}
          <article class="player-card" class:negative={p.profileTone === 'negative'} class:positive={p.profileTone === 'positive'}>
            <header>
              <div>
                <button class="name" on:click={() => openUrl(p.opggUrl)}>{p.riotId}</button>
                <div class="subline">{p.rankText} · {p.mainRole} · confidence {p.confidence}</div>
              </div>
              <span class="label">{p.profileLabel}</span>
            </header>
            <p class="summary">{p.summary}</p>
            <div class="champions">
              {#each p.champions.slice(0, 3) as c}
                <div class="champ">
                  <img src={c.iconUrl} alt={c.name} />
                  <span>{c.name}</span>
                  <small>{c.games}g · {c.wins}W</small>
                </div>
              {/each}
            </div>
            <div class="metrics">
              {#each Object.entries(p.metrics) as [k, v]}
                <span><b>{k}</b>{v}</span>
              {/each}
            </div>
            <div class="tags">
              {#each p.tags.slice(0, 5) as tag}<em>{tag}</em>{/each}
            </div>
            <footer>
              <button on:click={() => openUrl(p.opggUrl)}>OP.GG</button>
              <button on:click={() => openUrl(p.uggUrl)}>U.GG</button>
              <button on:click={() => openUrl(p.mmradarUrl)}>MMR</button>
              <button on:click={() => openUrl(p.deeplolUrl)}>DeepLOL</button>
            </footer>
          </article>
        {/if}
      {/each}
    </section>
  {/if}
</main>
