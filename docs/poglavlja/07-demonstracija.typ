= Демонстрација

У овом поглављу је приказан типичан ток коришћења платформе, од покретања система до прегледа предлога исправки, као и кратка демонстрација CLI алата. Снимци екрана су настали извршавањем описаног сценарија над покренутим системом.

== Припрема окружења

Систем се покреће једном командом из кореног директоријума репозиторијума:

```
docker compose up
```

Након што `docker compose` јави да су сви контејнери здрави (`healthy`) — редослед покретања је гарантован преко `depends_on` услова описаних у поглављу 3.5.3 — Веб интерфејс и API Gateway постају доступни.

== Пријава

На почетној страници корисник уноси креденцијале налога (Слика 2) и бира "Sign In". Захтев иде на `POST /api/auth/login` (поглавље 5.1.6), а по успешној пријави корисник се преусмерава на Dashboard, док се JWT токен чува у `localStorage` (поглавље 5.7.3).

#figure(
  image("/slike/slika7-1-prijava.png", width: 100%),
  caption: [Пријавна страница Веб интерфејса]
) <fig-2>

== Покретање анализе

На Dashboard-u се бира картица "Code Snippet" и језик Python, након чега се уноси исечак кода (Слика 3):

```python
import sqlite3

API_KEY = "sk_live_4f8a9b2c1d3e4f5a6b7c8d9e0f1a2b3c"

def get_user_orders(user_id):
    conn = sqlite3.connect("shop.db")
    cursor = conn.cursor()
    query = "SELECT * FROM orders WHERE user_id = " + user_id
    cursor.execute(query)
    return cursor.fetchall()

def build_report(order_ids):
    report = ""
    for oid in order_ids:
        report += "Order #" + str(oid) + "\n"
    return report
```

Овај пример намерно садржи три препознатљива проблема описана у поглављу 5.3: хардкодован API кључ (`security.hardcoded_secret`), SQL упит састављен конкатенацијом стрингова без параметризације (`security.sql_injection`, препознат од стране Semgrep-a, поглавље 5.3.6) и конкатенацију стрингова унутар петље (`performance.string_concat_in_loop`, поглавље 5.3.5). Након избора "Run Analysis", захтев се шаље на `POST /api/analyze` (поглавље 5.6.5), а корисник се преусмерава на страницу резултата за добијени `analysis_job_id`.

#figure(
  image("/slike/slika7-2-dashboard.png", width: 100%),
  caption: [Dashboard са унетим примером кода пре покретања анализе]
) <fig-3>

== Праћење напретка

Страница резултата одмах по отварању почиње периодично да позива `GET /api/results/:job_id` (поглавље 5.7.5) и приказује текст напретка који враћа API Gateway (Слика 4). Како пример садржи само један фајл, фаза обраде је кратка.

#figure(
  image("/slike/slika7-3-napredak.png", width: 100%),
  caption: [Праћење напретка анализе]
) <fig-4>

== Преглед резултата и предлог исправке

По завршетку посла, страница приказује кружни график расподеле проблема по озбиљности, листу проблема сортирану по `rank_score`, и за изабран проблем — објашњење и предлог исправке кроз Monaco `DiffEditor` (Слика 5, поглавље 5.7.5). Над примером из поглавља 7.3, систем је пронашао тачно три проблема: два оцењена као `Critical` (хардкодован кључ и SQL инјекција, коју је препознао Semgrep под правилом `python.sqlalchemy.security.sqlalchemy-execute-raw-query` — поглавље 5.3.8) и један као `Medium` (конкатенација стрингова у петљи).

За налаз `security.hardcoded_secret`, систем је доделио `rank_score` 65/100 ("Priority 65/100") и статичан `impact_score` 95/100 — потоња вредност се тачно поклапа са вредношћу из табеле 10 (поглавље 5.5.4). Предлог исправке (поглавље 5.5.3) упућује на читање тајне из environment променљиве уместо директног уписа у изворни код, уз текстуално објашњење ризика (`"Hardcoded secrets... are a critical security risk... Use environment variables or a secrets manager"`).

#figure(
  image("/slike/slika7-4-rezultati-predlog.jpeg", width: 100%),
  caption: [Преглед резултата]
) <fig-5>

== Историја анализа

Након извршене анализе, посао се појављује у историји корисника (`GET /api/jobs`, поглавље 5.6.7), са називом, статусом, језиком и временом извршавања (Слика 6).

#figure(
  image("/slike/slika7-6-istorija.png", width: 100%),
  caption: [Бочна трака са историјом анализа корисника]
) <fig-6>

// #todo[У тренутку писања овог поглавља, освежавање листе историје након завршетка посла (без ручног "Refresh" клика) било је у фази исправке, па је Слика 6 реконструисана ради илустрације очекиваног изгледа. Заменити стварним снимком екрана након потврде да се листа аутоматски освежава, или оставити као илустрацију уз кратку напомену уколико се проблем не стигне решити до предаје рада.]

Брисање посла (`DELETE /api/results/:job_id`, поглавље 5.6.8) се покреће из исте листе; након потврде, ставка нестаје из историје, а поновни покушај приступа истом `job_id`-у враћа грешку, с обзиром да су сви повезани записи у PostgreSQL, MongoDB и Redis бази обрисани.

== Анализа путем CLI алата

Исти пример кода, сачуван локално као `demo.py`, може се анализирати и без Веб интерфејса, коришћењем CLI алата описаног у поглављу 5.8. Поруке алата су на енглеском језику, доследно остатку корисничког интерфејса:

#figure(
```
$ repo-optimizer login
Email: user@example.com
Password: ****************
✓ Signed in. Token saved to ~/.repo-optimizer/config.toml

$ repo-optimizer analyze ./demo.py
✓ Packed 1 file, uploading...
⠋ Analyzing... (0/1 files processed)
⠙ Analyzing... (1/1 files processed)
✓ Analysis complete (job 753afa3c-...)

┌─────────────────────────────────────────────────────────────┐
│ demo.py                                                     │
├─────────────────────────────────────────────────────────────┤
│ [CRITICAL] security.hardcoded_secret            line 3      │
│   Hardcoded API key detected. Use environment variables...  │
│                                                             │
│ [CRITICAL] security.sql_injection               line 8      │
│   Avoiding SQL string concatenation...                      │
│                                                             │
│ [MEDIUM]   performance.string_concat_in_loop     line 14    │
│   String concatenation with `+=` inside a loop is O(n^2)... │
└─────────────────────────────────────────────────────────────┘

$ repo-optimizer analyze ./demo.py --json > result.json
```,
  caption: [Илустративан ток извршавања CLI алата над примером из поглавља 7.3]
) <lst-19>

Резултат је, с обзиром да CLI алат користи исте руте API Gateway-a као и Веб интерфејс (поглавље 5.8.1), идентичан ономе из поглавља 7.5 — разлика је искључиво у облику приказа (терминалски текст уместо графичког интерфејса), док је последња команда пример излаза погодног за даљу аутоматску обраду (`--json`, поглавље 5.8.6), нпр. у оквиру CI/CD цевовода.
