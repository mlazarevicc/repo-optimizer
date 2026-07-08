use crate::models::{Language, RankedProblemPayload, Suggestion};
use chrono::Utc;
use uuid::Uuid;

pub struct SuggestionEngine;

impl SuggestionEngine {
    pub fn new() -> Self { Self }

    pub fn generate(&self, analysis_job_id: Uuid, problem: &RankedProblemPayload) -> Suggestion {
        let p_type = problem.problem_type.to_lowercase();
        let lang = &problem.language;
        let snippet = &problem.code_snippet;

        let (explanation, suggested_code, impact_score) =
            self.build_suggestion(&p_type, lang, snippet);

        Suggestion {
            id: Uuid::new_v4(),
            problem_id: problem.id,
            analysis_job_id,
            problem_type: problem.problem_type.clone(),
            explanation,
            original_code: snippet.clone(),
            suggested_code,
            impact_score,
            created_at: Utc::now(),
        }
    }

    fn build_suggestion(
        &self,
        p_type: &str,
        lang: &Language,
        snippet: &str,
    ) -> (String, String, u8) {
        // ----------------------------------------------------------------
        // SECURITY
        // ----------------------------------------------------------------
        if p_type.contains("hardcoded_secret") || p_type.contains("hardcoded-password")
            || p_type.contains("credential")
        {
            let code = match lang {
                Language::Python =>
                    "import os\n\
                     # Citaj iz environment varijable, nikad hardkoduj\n\
                     secret = os.environ[\"SECRET_KEY\"]\n\
                     api_key = os.environ[\"API_KEY\"]",
                Language::JavaScript | Language::TypeScript =>
                    "// .env fajl + dotenv paket\n\
                     import 'dotenv/config';\n\
                     const secret = process.env.SECRET_KEY;\n\
                     const apiKey = process.env.API_KEY;",
                Language::Rust =>
                    "// Cargo.toml: dotenvy = \"0.15\"\n\
                     dotenvy::dotenv().ok();\n\
                     let secret = std::env::var(\"SECRET_KEY\")\n\
                         .expect(\"SECRET_KEY must be set\");",
                Language::Java =>
                    "// application.properties / Spring @Value\n\
                     @Value(\"${app.secret-key}\")\n\
                     private String secretKey;\n\
                     // ili direktno:\n\
                     String key = System.getenv(\"SECRET_KEY\");",
                Language::Go =>
                    "// os.Getenv ili godotenv paket\n\
                     import \"os\"\n\
                     secret := os.Getenv(\"SECRET_KEY\")\n\
                     if secret == \"\" { log.Fatal(\"SECRET_KEY not set\") }",
                _ =>
                    "# Citaj tajne iz environment varijabli:\n\
                     secret = os.environ[\"SECRET_KEY\"]",
            };
            return (
                "Hardcoded secrets (passwords, API keys) in source code are a critical \
                 security risk. They end up in version control, logs, and error messages. \
                 Use environment variables or a secrets manager (e.g. AWS Secrets Manager, \
                 HashiCorp Vault, Azure Key Vault).".into(),
                code.into(),
                95,
            );
        }

        if p_type.contains("sql_injection") || p_type.contains("sqlalchemy-execute-raw")
        {
            let code = match lang {
                Language::Python =>
                    "# Parametrizovane upite uvek - nikad string konkatenacija\n\
                     cursor.execute(\n\
                         \"SELECT * FROM users WHERE id = %s\",\n\
                         (user_id,)  # tuple sa parametrima\n\
                     )\n\
                     # SQLAlchemy ORM:\n\
                     user = session.get(User, user_id)",
                Language::JavaScript | Language::TypeScript =>
                    "// Prepared statements sa pg/mysql2:\n\
                     const result = await pool.query(\n\
                        'SELECT * FROM users WHERE id = $1',\n\
                        [userId]\n\
                     );\n\
                     // Prisma/TypeORM ORM-i su bezbedni po defaultu",
                Language::Rust =>
                    "// sqlx automatski sanitizuje parametre:\n\
                     let user = sqlx::query_as::<_, User>(\n\
                         \"SELECT * FROM users WHERE id = $1\"\n\
                     )\n\
                     .bind(user_id)\n\
                     .fetch_one(&pool).await?;",
                Language::Java =>
                    "// PreparedStatement - nikad Statement + konkatenacija\n\
                     PreparedStatement ps = conn.prepareStatement(\n\
                         \"SELECT * FROM users WHERE id = ?\"\n\
                     );\n\
                     ps.setLong(1, userId);\n\
                     ResultSet rs = ps.executeQuery();",
                Language::Go =>
                    "// database/sql - placeholder ? ili $1\n\
                     row := db.QueryRow(\n\
                         \"SELECT * FROM users WHERE id = $1\", userId,\n\
                     )\n\
                     // GORM je bezbedan po defaultu",
                _ =>
                    "// Uvek koristi parametrizovane upite:\n\
                     db.query(\"SELECT * FROM users WHERE id = $1\", [userId])",
            };
            return (
                "SQL Injection is a critical vulnerability. String concatenation to build \
                 queries allows attackers to manipulate the query and read, modify or delete \
                 any data in the database. Always use parameterized queries / prepared statements.".into(),
                code.into(),
                90,
            );
        }

        if p_type.contains("xss") {
            let code = match lang {
                Language::JavaScript | Language::TypeScript =>
                    "// Nikad innerHTML sa korisnickim unosom\n\
                     // Koristiti textContent (automatski escapuje HTML)\n\
                     element.textContent = userInput;\n\
                     // Ili DOMPurify za rich text:\n\
                     import DOMPurify from 'dompurify';\n\
                     element.innerHTML = DOMPurify.sanitize(userInput);",
                _ =>
                    "// Escapuj HTML specijalnih karaktera (<, >, \", ', &)\n\
                     // pre ubacivanja korisnickog sadrzaja u DOM.",
            };
            return (
                "Cross-Site Scripting (XSS) allows attackers to inject malicious scripts \
                 into your pages that then run in victims' browsers — stealing cookies, \
                 session tokens, or performing actions on behalf of the user. \
                 Never insert unsanitized user input directly into the DOM.".into(),
                code.into(),
                85,
            );
        }

        if p_type.contains("insecure_random") {
            let code = match lang {
                Language::Python =>
                    "import secrets\n\
                     # Za tokene i session ID-ove:\n\
                     token = secrets.token_hex(32)         # 64 hex znaka\n\
                     token = secrets.token_urlsafe(32)     # URL-safe base64\n\
                     otp = str(secrets.randbelow(1_000_000)).zfill(6)  # 6-cifarski OTP",
                Language::JavaScript | Language::TypeScript =>
                    "// Browser:\n\
                     const arr = new Uint8Array(32);\n\
                     crypto.getRandomValues(arr);\n\
                     const token = Buffer.from(arr).toString('hex');\n\
                     // Node.js:\n\
                     import { randomBytes } from 'node:crypto';\n\
                     const token = randomBytes(32).toString('hex');",
                Language::Rust =>
                    "// Dodati u Cargo.toml: rand = { features = [\"getrandom\"] }\n\
                     use rand::RngCore;\n\
                     let mut buf = [0u8; 32];\n\
                     rand::rngs::OsRng.fill_bytes(&mut buf);\n\
                     let token = hex::encode(buf);",
                Language::Java =>
                    "import java.security.SecureRandom;\n\
                     SecureRandom rng = new SecureRandom();\n\
                     byte[] buf = new byte[32];\n\
                     rng.nextBytes(buf);\n\
                     String token = HexFormat.of().formatHex(buf);",
                Language::Go =>
                    "import \"crypto/rand\"\n\
                     buf := make([]byte, 32)\n\
                     if _, err := rand.Read(buf); err != nil {\n\
                         return err\n\
                     }\n\
                     token := hex.EncodeToString(buf)",
                _ =>
                    "// Koristi kriptografski siguran generator slucajnih brojeva\n\
                     // umesto standardnog PRNG-a.",
            };
            return (
                "Math.random() / Python random module use a Pseudo-Random Number Generator \
                 (PRNG) that is predictable — an attacker who observes output values can \
                 predict future ones. Never use it for tokens, session IDs, OTPs, salts or \
                 passwords. Use a Cryptographically Secure PRNG (CSPRNG) instead.".into(),
                code.into(),
                88,
            );
        }

        // ----------------------------------------------------------------
        // PERFORMANCE
        // ----------------------------------------------------------------
        if p_type.contains("n_plus_one") || p_type.contains("unoptimized_query") {
            let code = match lang {
                Language::Python =>
                    "# Umesto N+1 upita u petlji:\n\
                     # for user in users: order = db.query('SELECT ... WHERE user_id=?', user.id)\n\
                     #\n\
                     # Jedan batch upit:\n\
                     user_ids = [u.id for u in users]\n\
                     orders = db.query(\n\
                         'SELECT * FROM orders WHERE user_id = ANY(%s)', (user_ids,)\n\
                     )\n\
                     # Ili SQLAlchemy eager loading:\n\
                     users = session.query(User).options(joinedload(User.orders)).all()",
                Language::JavaScript | Language::TypeScript =>
                    "// Prisma - eager loading:\n\
                     const users = await prisma.user.findMany({\n\
                       include: { orders: true },\n\
                     });\n\
                     // TypeORM:\n\
                     const users = await User.find({ relations: ['orders'] });",
                Language::Rust =>
                    "// sqlx - jedan JOIN upit umesto petlje:\n\
                     let rows = sqlx::query!(\n\
                          \"SELECT u.*, o.id as order_id FROM users u \
                          LEFT JOIN orders o ON o.user_id = u.id\"\n\
                     )\n\
                     .fetch_all(&pool).await?;",
                Language::Java =>
                    "// JPA - @OneToMany(fetch = FetchType.EAGER)\n\
                     // ili JPQL JOIN FETCH:\n\
                     @Query(\"SELECT u FROM User u JOIN FETCH u.orders\")\n\
                     List<User> findAllWithOrders();",
                Language::Go =>
                    "// Jedan JOIN umesto N upita:\n\
                     rows, _ := db.Query(\n\
                          `SELECT u.*, o.id FROM users u \
                          LEFT JOIN orders o ON o.user_id = u.id`,\n\
                     )",
                _ =>
                    "// Koristi batch/JOIN upit umesto pojedinacnih upita u petlji.",
            };
            return (
                "N+1 query problem: executing one database query per iteration of a loop \
                 means N queries instead of 1. For 1000 users, that's 1000 round trips to \
                 the database. Use a single batch query, JOIN, or ORM eager loading.".into(),
                code.into(),
                80,
            );
        }

        if p_type.contains("string_concat_in_loop") || p_type.contains("string_concat") {
            let code = match lang {
                Language::Python =>
                    "# O(n²) - svaka += kreira novi string:\n\
                     # for item in items: result += str(item)\n\
                     #\n\
                     # O(n) - skupi u listu, join na kraju:\n\
                     parts = [str(item) for item in items]\n\
                     result = \"\".join(parts)\n\
                     # Ili jednom linijom:\n\
                     result = \"\".join(str(item) for item in items)",
                Language::JavaScript | Language::TypeScript =>
                    "// O(n²):\n\
                     // for (const item of items) { html += `<li>${item}</li>`; }\n\
                     //\n\
                     // O(n) - Array.push + join:\n\
                     const parts: string[] = [];\n\
                     for (const item of items) {\n\
                        parts.push(`<li>${item}</li>`);\n\
                     }\n\
                     const html = parts.join('');\n\
                     // Ili u jednom redu:\n\
                     const html = items.map(i => `<li>${i}</li>`).join('');",
                Language::Rust =>
                    "// Rust String += je O(1) amortizovano za String tip,\n\
                     // ali ako formatiras mnogo delova, String::with_capacity + push_str je bolje:\n\
                     let mut buf = String::with_capacity(items.len() * 20);\n\
                     for item in &items {\n\
                          buf.push_str(&item.to_string());\n\
                     }\n\
                     // Ili uz iteratore:\n\
                     let result: String = items.iter().map(|i| i.to_string()).collect();",
                Language::Java =>
                    "// O(n²) - String += u petlji:\n\
                     // for (Item i : items) { result += i; }\n\
                     //\n\
                     // O(n) - StringBuilder:\n\
                     StringBuilder sb = new StringBuilder();\n\
                     for (Item item : items) {\n\
                          sb.append(item.toString());\n\
                     }\n\
                     String result = sb.toString();\n\
                     // Ili Stream.collect(Collectors.joining()):",
                Language::Go =>
                    "// strings.Builder - O(n):\n\
                     var sb strings.Builder\n\
                     for _, item := range items {\n\
                          sb.WriteString(item.String())\n\
                     }\n\
                     result := sb.String()\n\
                     // Ili strings.Join za slice of strings:\n\
                     result := strings.Join(parts, \"\")",
                _ =>
                    "// Skupi delove u listu/buffer, ne koristi += u petlji.\n\
                     // Tada napravi konacni string jednom operacijom (join, toString...).",
            };
            return (
                "String concatenation with `+=` inside a loop is O(n²): each iteration \
                 allocates a new string and copies all previous content. For n iterations \
                 of average length k, that's n×(n×k)/2 bytes total. \
                 Collect parts into a list/buffer and join at the end — one allocation, O(n).".into(),
                code.into(),
                72,
            );
        }

        if p_type.contains("nested_loop") || p_type.contains("unoptimized_loop") {
            let code = match lang {
                Language::Python =>
                    "# Zameni O(n²) nested loop sa HashMap lookupom O(n):\n\
                     # Umesto: for a in list_a: for b in list_b: if a.id == b.id: ...\n\
                     lookup = {b.id: b for b in list_b}   # O(n) jednom\n\
                     for a in list_a:\n\
                          b = lookup.get(a.id)            # O(1) po iteraciji\n\
                          if b: ...",
                Language::JavaScript | Language::TypeScript =>
                    "// O(n²) -> O(n) sa Map:\n\
                     const lookup = new Map(listB.map(b => [b.id, b]));\n\
                     for (const a of listA) {\n\
                        const b = lookup.get(a.id);   // O(1)\n\
                        if (b) { /* ... */ }\n\
                     }",
                Language::Rust =>
                    "// HashMap lookup umesto nested iterator:\n\
                     use std::collections::HashMap;\n\
                     let lookup: HashMap<_, _> = list_b.iter().map(|b| (b.id, b)).collect();\n\
                     for a in &list_a {\n\
                          if let Some(b) = lookup.get(&a.id) { /* ... */ }\n\
                     }",
                Language::Java =>
                    "// HashMap umesto nested for:\n\
                     Map<Long, B> lookup = listB.stream()\n\
                          .collect(Collectors.toMap(B::getId, b -> b));\n\
                     for (A a : listA) {\n\
                          B b = lookup.get(a.getId());\n\
                          if (b != null) { /* ... */ }\n\
                     }",
                Language::Go =>
                    "// map lookup umesto nested range:\n\
                     lookup := make(map[int64]*B, len(listB))\n\
                     for _, b := range listB { lookup[b.ID] = b }\n\
                     for _, a := range listA {\n\
                          if b, ok := lookup[a.ID]; ok { /* ... */ }\n\
                     }",
                _ =>
                    "// Zameni nested loop sa hash map lookupom: O(n) umesto O(n²).",
            };
            return (
                "Nested loops over the same or related collections cause O(n²) or worse \
                 complexity. For 1000 items, that's 1,000,000 iterations instead of 1,000. \
                 Use a HashMap/Dictionary for O(1) lookups inside the outer loop.".into(),
                code.into(),
                75,
            );
        }

        // ----------------------------------------------------------------
        // CODE SMELLS
        // ----------------------------------------------------------------
        if p_type.contains("deep_nesting") || p_type.contains("long_parameter_list") {
            let code = match lang {
                Language::Python =>
                    "# Koristiti early return / guard clause umesto dubokog ugnezdavanja:\n\
                     def process(user, config, data):\n\
                          if not user: return None   # guard\n\
                          if not config.enabled: return None\n\
                          # ostatak logike na jednom nivou uvlacenja\n\
                          return do_work(user, data)",
                Language::JavaScript | Language::TypeScript =>
                    "// Guard clauses - early return:\n\
                     function process(user, config, data) {\n\
                        if (!user) return null;\n\
                        if (!config.enabled) return null;\n\
                        // ostatak logike na jednom nivou\n\
                        return doWork(user, data);\n\
                     }",
                Language::Rust =>
                    "// Early return sa ? operator ili if-let:\n\
                     fn process(user: Option<&User>, config: &Config) -> Option<Result> {\n\
                          let user = user?;           // early return ako None\n\
                          if !config.enabled { return None; }\n\
                          Some(do_work(user, config))\n\
                     }",
                Language::Java =>
                    "// Guard clauses:\n\
                     public Result process(User user, Config config) {\n\
                          if (user == null) return null;\n\
                          if (!config.isEnabled()) return null;\n\
                          return doWork(user, config);\n\
                     }",
                Language::Go =>
                    "// Go idiom - early error return:\n\
                     func process(user *User, cfg *Config) (*Result, error) {\n\
                          if user == nil { return nil, ErrNoUser }\n\
                          if !cfg.Enabled { return nil, ErrDisabled }\n\
                          return doWork(user, cfg)\n\
                     }",
                _ =>
                    "// Zameni duboko ugnezdavanje sa early return / guard clause pattern-om.",
            };
            return (
                "Deep nesting makes code hard to read and test. Each level of indentation \
                 adds cognitive overhead for the reader. Use early returns (guard clauses) \
                 to handle edge cases at the top of the function and keep the main \
                 logic at a flat indentation level.".into(),
                code.into(),
                65,
            );
        }

        if p_type.contains("long_method") || p_type.contains("complex_method") {
            let code = match lang {
                Language::Python =>
                    "# Podeli veliku funkciju na manje, svaka sa jednom odgovornoscu:\n\
                     def process_order(order):\n\
                          validated = _validate_order(order)\n\
                          priced = _apply_pricing(validated)\n\
                          return _save_order(priced)\n\
                     \n\
                     def _validate_order(order): ...\n\
                     def _apply_pricing(order): ...\n\
                     def _save_order(order): ...",
                Language::JavaScript | Language::TypeScript =>
                    "// Decompose u manje funkcije:\n\
                     function processOrder(order) {\n\
                        const validated = validateOrder(order);\n\
                        const priced = applyPricing(validated);\n\
                        return saveOrder(priced);\n\
                     }",
                Language::Rust =>
                    "// Razbij na privatne fn-ove:\n\
                     pub fn process_order(order: Order) -> Result<SavedOrder> {\n\
                          let validated = validate_order(order)?;\n\
                          let priced = apply_pricing(validated)?;\n\
                          save_order(priced)\n\
                     }",
                Language::Java =>
                    "// Single Responsibility - ekstraktuj private metode:\n\
                     public SavedOrder processOrder(Order order) {\n\
                          Order validated = validateOrder(order);\n\
                          Order priced = applyPricing(validated);\n\
                          return saveOrder(priced);\n\
                     }",
                Language::Go =>
                    "// Izdvoj u zasebne funkcije:\n\
                     func processOrder(order Order) (SavedOrder, error) {\n\
                          validated, err := validateOrder(order)\n\
                          if err != nil { return SavedOrder{}, err }\n\
                          priced, err := applyPricing(validated)\n\
                          if err != nil { return SavedOrder{}, err }\n\
                          return saveOrder(priced)\n\
                     }",
                _ =>
                    "// Podeli veliku funkciju prema Single Responsibility principu.",
            };
            return (
                "This function is too long and handles too many responsibilities, \
                 violating the Single Responsibility Principle. Long functions are hard \
                 to test, hard to understand, and fragile to change. Extract logical \
                 segments into well-named helper functions.".into(),
                code.into(),
                60,
            );
        }

        if p_type.contains("duplicate_code") || p_type.contains("duplication") {
            let code = match lang {
                Language::Python =>
                    "# Izdvoji duplikovanu logiku u zajednicku funkciju:\n\
                     def shared_logic(price, quantity, discount_rate):\n\
                          subtotal = price * quantity\n\
                          return subtotal * (1 - discount_rate)\n\
                     \n\
                     # Pozovi sa oba mesta:\n\
                     result_a = shared_logic(price_a, qty_a, rate)\n\
                     result_b = shared_logic(price_b, qty_b, rate)",
                Language::JavaScript | Language::TypeScript =>
                    "// Zajednička funkcija umesto kopiranja:\n\
                     function calcTotal(price, qty, discount) {\n\
                        return price * qty * (1 - discount);\n\
                     }\n\
                     const totalA = calcTotal(priceA, qtyA, rate);\n\
                     const totalB = calcTotal(priceB, qtyB, rate);",
                Language::Rust =>
                    "// Zajednička fn (ili metoda na trait-u):\n\
                     fn calc_total(price: f64, qty: u32, discount: f64) -> f64 {\n\
                          price * qty as f64 * (1.0 - discount)\n\
                     }",
                Language::Java =>
                    "// Privatan helper metod ili utility klasa:\n\
                     private double calcTotal(double price, int qty, double discount) {\n\
                          return price * qty * (1 - discount);\n\
                     }",
                Language::Go =>
                    "// Zajednička funkcija:\n\
                     func calcTotal(price float64, qty int, discount float64) float64 {\n\
                          return price * float64(qty) * (1 - discount)\n\
                     }",
                _ =>
                    "// Izdvoji duplikovanu logiku u zajednicku funkciju i pozovi je sa oba mesta.",
            };
            return (
                "Duplicated code is a maintenance hazard: a bug fixed in one copy is \
                 silently left in the other. It also inflates the codebase and makes \
                 changes harder. Extract the shared logic into a reusable function \
                 (DRY — Don't Repeat Yourself).".into(),
                code.into(),
                80,
            );
        }

        if p_type.contains("magic_number") {
            let code = match lang {
                Language::Python =>
                    "# Loše:\n\
                     if elapsed > 86400:\n\
                          expire_session()\n\
                     \n\
                     # Dobro - named constant:\n\
                     SESSION_EXPIRY_SECONDS = 24 * 60 * 60  # 24h\n\
                     if elapsed > SESSION_EXPIRY_SECONDS:\n\
                          expire_session()",
                Language::JavaScript | Language::TypeScript =>
                    "// Named constant:\n\
                     const SESSION_EXPIRY_MS = 24 * 60 * 60 * 1000; // 24h\n\
                     if (elapsed > SESSION_EXPIRY_MS) expireSession();",
                Language::Rust =>
                    "const SESSION_EXPIRY_SECS: u64 = 24 * 60 * 60; // 24h\n\
                     if elapsed > SESSION_EXPIRY_SECS { expire_session(); }",
                Language::Java =>
                    "private static final int SESSION_EXPIRY_SECONDS = 86_400; // 24h\n\
                     if (elapsed > SESSION_EXPIRY_SECONDS) expireSession();",
                Language::Go =>
                    "const SessionExpirySeconds = 24 * 60 * 60 // 24h\n\
                     if elapsed > SessionExpirySeconds { expireSession() }",
                _ =>
                    "// Imenuj magicni broj kao konstantu (SCREAMING_CASE) sa komentarom.",
            };
            return (
                "Magic numbers make code hard to understand and maintain. \
                 A reader seeing `86400` must guess what it means; \
                 `SESSION_EXPIRY_SECONDS = 86400` is self-documenting. \
                 Named constants also make changes safer: update the value in one place.".into(),
                code.into(),
                55,
            );
        }

        if p_type.contains("large_class") || p_type.contains("long_file") {
            let code = match lang {
                Language::Python =>
                    "# Podeli veliku klasu po odgovornostima (SRP):\n\
                     # Umesto: class GodClass: # 2000 linija\n\
                     class UserAuth: ...       # autentifikacija\n\
                     class UserProfile: ...    # profil\n\
                     class UserNotifications: ... # notifikacije",
                Language::JavaScript | Language::TypeScript =>
                    "// Podeli po odgovornostima u zasebne fajlove:\n\
                     // auth.service.ts, profile.service.ts, notifications.service.ts\n\
                     export class AuthService { ... }\n\
                     export class ProfileService { ... }",
                Language::Rust =>
                    "// Podeli u zasebne module i impl blokove:\n\
                     mod auth;    // auth.rs\n\
                     mod profile; // profile.rs\n\
                     // Svaki modul ima svoju impl User { ... }",
                Language::Java =>
                    "// Podeli po SRP principu:\n\
                     public class UserAuthService { ... }\n\
                     public class UserProfileService { ... }\n\
                     public class NotificationService { ... }",
                Language::Go =>
                    "// Podeli u zasebne struct-ove / fajlove:\n\
                     type AuthService struct { ... }\n\
                     type ProfileService struct { ... }",
                _ =>
                    "// Podeli veliku klasu/fajl prema Single Responsibility Principle.",
            };
            return (
                "This class/file is too large and handles too many responsibilities, \
                 making it a 'God Class'. It's hard to test, understand, and modify \
                 without breaking something. Apply the Single Responsibility Principle \
                 and split into smaller, focused classes/modules.".into(),
                code.into(),
                58,
            );
        }

        // ----------------------------------------------------------------
        // FALLBACK - genericni predlog
        // ----------------------------------------------------------------
        (
            format!(
                "This code section needs refactoring to improve quality, security, or \
                 readability. Problem type detected: `{}` in {} code.",
                p_type,
                lang.display_name()
            ),
            format!(
                "// Konsultuj best practices za {} kod:\n\
                 // https://docs.rs / PEP8 / Google Style Guide\n\
                 // za tip problema: {}",
                lang.display_name(), p_type
            ),
            50,
        )
    }
}
