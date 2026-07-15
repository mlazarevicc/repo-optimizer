# RepoOptimizer

> Distribuirana platforma za statičku analizu i optimizaciju koda, sa podrškom za više programskih jezika

**Status**: Aktivan razvoj | **Jezik**: Rust | **Licenca**: MIT

---

## Sadržaj

- [Pregled](#pregled)
- [Funkcionalnosti](#funkcionalnosti)
- [Arhitektura](#arhitektura)
- [Tehnologije](#tehnologije)
- [Trenutno stanje implementacije](#trenutno-stanje-implementacije)
- [Pravci daljeg razvoja](#pravci-daljeg-razvoja)

---

## Pregled

**RepoOptimizer** je distribuirani sistem za automatizovanu statičku analizu koda kroz više programskih jezika. Cilj platforme je da programerima brzo ukaže na probleme u kodu koji najviše utiču na kvalitet i održivost projekta:

- **Performanse**: neefikasni algoritmi, N+1 problemi, nepotrebno kopiranje/alokacije
- **Bezbednost**: SQL injekcije, nedostatak validacije ulaza, hard-kodovani kredencijali
- **Code smell obrasci**: predugačke metode, duboko ugnježdavanje, duplirani kod
- **Dobre prakse**: konvencije imenovanja, kompleksnost funkcija, nedostatak dokumentacije

### Problem koji se rešava

Ručni code review je vremenski zahtevan, a postojeći alati su često vezani za jedan jezik, kompleksni za integraciju ili daju samo generičko upozorenje bez konkretnog predloga rešenja. RepoOptimizer pokušava da ovo prevaziđe kroz:

- podršku za više jezika kroz zajednički Tree-sitter sloj za parsiranje,
- jednostavnu integraciju (Web UI, u planu i CLI),
- konkretne predloge ispravki, a ne samo listu problema.

---

## Funkcionalnosti

### Parsiranje koda (Tree-sitter)

Parser servis generiše AST za svaki podržani jezik: **Python, JavaScript, Go, Java i Rust**. AST se čuva u MongoDB i predstavlja osnovu za dalju analizu.

### Statička analiza

Analysis servis nad AST-om primenjuje skup pravila organizovanih u četiri kategorije:

- **Code smells**: duge metode, velike klase, duboko ugnježdavanje, duplirani blokovi koda
- **Performanse**: neefikasne petlje, sumnjiva vremenska složenost
- **Bezbednost**: SQL injekcije, hard-kodovani kredencijali, nedostatak validacije ulaza
- **Stil**: konvencije imenovanja, osnovna kompleksnost funkcija

Skup pravila trenutno nije podjednako razvijen za sve jezike (npr. detekcija SQL injekcije za Javu je u planu, ali još nije implementirana). Tamo gde se preklapaju rezultati Semgrep-a i internih detektora, primenjuje se deduplikacija kako korisnik ne bi video duplirane nalaze.

### Rangiranje problema

ML Ranker servis dodeljuje težinu (severity) svakom pronađenom problemu na osnovu njegovog tipa, po heurističkom, pravilima definisanom modelu (rule-based baseline). Rezultati se keširaju u Redisu radi bržeg ponovnog pristupa. Zamena ovog modela pravim ML modelom (npr. XGBoost/ONNX) je navedena kao mogući pravac daljeg razvoja, ne kao trenutna funkcionalnost.

### Generisanje predloga

Suggestion Generator servis, na osnovu tipa detektovanog problema, generiše predlog ispravke korišćenjem unapred definisanih (deterministic) templejta po jeziku i tipu problema. Predlozi su namerno jednostavni i predvidivi — ne koriste se LLM modeli niti kompleksno parsiranje konteksta.

### Web interfejs

React/Vite aplikacija sa Tailwind stilizacijom omogućava registraciju i prijavu korisnika, pokretanje analize i pregled rezultata (Dashboard, Login, Register, Result stranice), uz CodeMirror prikaz koda sa isticanjem sintakse.

---

## Arhitektura

### Pregled sistema

```
Web UI
    ↓
API Gateway (Actix-web, REST API, JWT middleware)
    ↓
RabbitMQ (asinhrona komunikacija između servisa)
    ↓
Parser Service (Tree-sitter → AST)
    ↓
Analysis Service (statička analiza nad AST-om)
    ↓
ML Ranker Service (rangiranje po težini)
    ↓
Suggestion Generator Service (generisanje predloga)
    ↓
API Gateway (agregacija i vraćanje rezultata)
    ↓
Web UI (prikaz rezultata)
```

### Mikroservisi

| Servis | Odgovornost | Tehnologije | Baza podataka |
|---|---|---|---|
| **Auth Service** | Registracija, prijava, JWT tokeni | Actix-web, JWT | PostgreSQL |
| **Parser Service** | Parsiranje koda u AST za više jezika | Tree-sitter, Tokio | MongoDB |
| **Analysis Service** | Statička analiza, detekcija problema | Custom pravila, Semgrep, Tokio | PostgreSQL |
| **ML Ranker Service** | Rangiranje problema po težini | Rule-based baseline | Redis (keš) |
| **Suggestion Generator Service** | Generisanje predloga ispravki | Template engine | MongoDB |
| **API Gateway** | REST API, autentikacija, kompozicija odgovora | Actix-web, JWT middleware | — |

Svi servisi komuniciraju asinhrono preko RabbitMQ, što omogućava nezavisno skaliranje i otpornost na privremeni pad pojedinačnog servisa.

### Tok podataka

```
Web UI → API Gateway (autentikacija)
       → RabbitMQ: kreiranje "analyze_code" posla
       → Parser Service: generisanje AST-a (→ MongoDB)
       → Analysis Service: primena pravila (→ PostgreSQL)
       → ML Ranker Service: rangiranje po težini (→ Redis keš)
       → Suggestion Generator Service: generisanje predloga (→ MongoDB)
       → API Gateway: agregacija rezultata
       → Web UI: prikaz korisniku
```

---

## Tehnologije

### Backend
- **Jezik**: Rust
- **Web framework**: Actix-web
- **Asinhroni runtime**: Tokio
- **Serijalizacija**: Serde
- **Pristup bazama**: SQLx (PostgreSQL), zvanični MongoDB drajver

### Obrada podataka
- **Parsiranje koda**: Tree-sitter
- **Message broker**: RabbitMQ
- **Keširanje**: Redis

### Frontend
- **Framework**: React 18+
- **Editor koda**: CodeMirror 6
- **Build alat**: Vite
- **Stilizacija**: Tailwind CSS
- **HTTP klijent**: Axios

### DevOps
- **Kontejnerizacija**: Docker, Docker Compose
- **Orkestracija**: Docker Compose za lokalno pokretanje i demonstraciju

### Kvalitet koda
- **Testiranje**: Rust-ov ugrađeni test framework (62 jedinična testa)
- **Dokumentacija**: cargo doc

---

## Trenutno stanje implementacije

Radi transparentnosti, ovde je pregled šta je od navedenih funkcionalnosti zaista implementirano u trenutnoj fazi projekta:

| Deo sistema | Status |
|---|---|
| API Gateway (rute, JWT middleware) | ✅ Implementirano |
| API Gateway — rate limiting | ⏳ Nije implementirano |
| Auth Service (JWT, PostgreSQL) | ✅ Implementirano |
| Parser Service (Tree-sitter, MongoDB) | ✅ Implementirano |
| Analysis Service (pravila, PostgreSQL) | ✅ Implementirano (nepotpun skup pravila po jeziku) |
| ML Ranker — rule-based rangiranje + Redis keš | ✅ Implementirano |
| ML Ranker — pravi ML model (XGBoost/ONNX) | ⏳ Nije implementirano (planirano kao budući rad) |
| Suggestion Generator — deterministički templejti | ✅ Implementirano |
| Suggestion Generator — LLM integracija | ⏳ Nije implementirano (budući rad) |
| RabbitMQ komunikacija između servisa | ✅ Implementirano |
| PostgreSQL i MongoDB | ✅ Konfigurisano i u upotrebi |
| Web UI (Dashboard, Login, Register, Result) | ✅ Implementirano |
| CLI alat | ⏳ U planu, sledeći korak razvoja |
| Docker Compose (kompletan sistem, `docker compose up`) | ✅ Implementirano |
| Kubernetes | 💭 Samo koncept za budući rad, ne koristi se u praksi |

---

## Pravci daljeg razvoja

Ovi pravci su navedeni kao mogući nastavak rada nakon odbrane, a ne kao deo trenutnog obima diplomskog rada:

- **CLI alat** — `repo-optimizer analyze <putanja>` za lokalnu analizu bez Web UI-ja (sledeći planirani korak)
- **Proširenje pravila analize** — dodatna pravila po jeziku (npr. SQL injekcija za Javu), radi ujednačavanja pokrivenosti
- **Pravi ML model za rangiranje** — zamena rule-based pristupa treniranim modelom (npr. XGBoost) na osnovu skupa obeleženih problema
- **LLM integracija za generisanje predloga** — kao mogućnost za naprednije, kontekstualne predloge ispravki umesto fiksnih templejta
- **Rate limiting na API Gateway-u**
- **Kubernetes orkestracija** — kao koncept skaliranja za produkcioni scenario, van okvira lokalnog demoa

---

## Bezbednost (trenutno stanje)

- JWT tokeni za autentikaciju korisnika
- Parametrizovani upiti u bazu radi sprečavanja SQL injekcija u okviru samog sistema
- Kredencijali i konfiguracija se čuvaju kroz environment varijable, ne u kodu

Napredne teme poput HTTPS terminacije, enkripcije podataka u mirovanju, GDPR i SOC 2 usklađenosti nisu deo trenutne implementacije i namerno su izostavljene iz opisa kako bi README odražavao realno stanje projekta.

---

**Autor: Milan Lazarevic**

Poslednja izmena: Jul 2026
