# revwlol / GIVERSS Dodge Assistant

Desktopowa wersja GIVERSS Dodge Assistant dla League of Legends.

Projekt jest przygotowany pod repozytorium `giveqq/revwlol` i zawiera automatyczny build Windows przez GitHub Actions.

## Co robi aplikacja

- działa jako normalna aplikacja desktopowa Windows,
- ma UI w stylu strony XAMPP: Kassadin background, rain effect, kompaktowe karty graczy,
- używa Riot API do analizy SoloQ,
- używa Riot ID / PUUID jako głównej identyfikacji graczy,
- liczy prosty status lobby: `Winnable`, `Playable`, `Dodge consider`,
- pokazuje main role, main championy, recent form, role consistency, comfort picki i krótkie zdanie o graczu,
- próbuje odczytać lokalnego klienta LoL przez LCU read-only,
- próbuje wykryć teammate'ów w champ select podobnie do aplikacji typu Reveal, ale bez automatyzacji klienta.

## Czego aplikacja NIE robi

- nie robi auto-accept,
- nie robi auto-dodge,
- nie pickuje championów,
- nie banuje,
- nie pisze na czacie,
- nie wysyła POST/PATCH do klienta LoL,
- nie czyta pamięci procesu gry.

Moduł klienta LoL jest zaprojektowany jako read-only.

## Build Windows EXE przez GitHub Actions

Po wrzuceniu projektu do repo:

1. Wejdź w zakładkę **Actions**.
2. Wybierz workflow **Build Windows EXE**.
3. Kliknij **Run workflow**.
4. Poczekaj kilka minut.
5. Pobierz artifact: **GIVERSS-Dodge-Assistant-Portable**.
6. W środku będzie `GIVERSS-Dodge-Assistant.exe`.

Nie musisz mieć Node.js ani Rust na swoim komputerze, jeśli korzystasz z builda przez GitHub Actions.

## Uruchomienie wersji developerskiej lokalnie

Wymagania lokalne:

- Node.js LTS,
- Rust stable,
- Visual Studio Build Tools / Windows SDK,
- Riot API key.

```bash
npm install
npm run desktop:dev
```

Build lokalny:

```bash
npm run desktop:build
```

Wynik będzie w:

```txt
src-tauri/target/release/bundle/
```

## Użycie

1. Odpal aplikację.
2. W ustawieniach wpisz Riot API key.
3. Wpisz swój Riot ID, np. `Name#TAG`.
4. Wybierz region/platformę.
5. Możesz wkleić OP.GG multisearch albo zwykłe Riot ID.
6. Możesz też odpalić klienta LoL i wejść w champ select — aplikacja spróbuje wykryć teammate'ów przez LCU.

## Lockfile

Aplikacja próbuje sama znaleźć:

```txt
C:\Riot Games\League of Legends\lockfile
```

Jeśli nie znajdzie, wpisz ścieżkę ręcznie w ustawieniach.

## Status projektu

To jest prototyp desktopowy. Główna logika jest przeniesiona z działającej wersji web/XAMPP, ale LCU/reveal-like część może wymagać poprawek po testach na realnym champ select.
