# GIVERSS Dodge Assistant — Windows portable build

Ten pakiet jest przygotowany tak, żeby GitHub sam skompilował aplikację na Windowsie. Dzięki temu na Twoim komputerze nie musisz instalować Node.js, Rust ani Tauri tylko po to, żeby dostać gotowy `.exe`.

## Jak zbudować `.exe` przez GitHub

1. Utwórz prywatne repozytorium na GitHubie.
2. Wrzuć do niego całą zawartość tego folderu.
3. Wejdź w zakładkę **Actions**.
4. Wybierz workflow **Build Windows Portable**.
5. Kliknij **Run workflow**.
6. Po zakończeniu wejdź w wykonany workflow i pobierz artifact:
   - `GIVERSS-Dodge-Assistant-Portable`

W środku będzie:

```txt
GIVERSS-Dodge-Assistant.exe
```

To jest wersja bez instalatora. Uruchamiasz ją bezpośrednio.

## Uwaga

Jeżeli Windows Defender pokaże ostrzeżenie, to dlatego, że plik nie jest podpisany certyfikatem code-signing. To normalne przy prywatnych buildach.

## Lokalny build na Twoim PC

Jeżeli chcesz budować lokalnie, potrzebujesz:

- Node.js LTS
- Rust/Cargo
- Microsoft C++ Build Tools
- WebView2 Runtime

Potem:

```bash
npm install
npm run desktop:build
```

Gotowy plik będzie w:

```txt
src-tauri/target/release/giverss-dodge-assistant.exe
```
