#let format_strane = "iso-b5"         // могуће вредности: iso-b5, a4
#let naslov = "RepoOptimizer - дистрибуирани систем за статичку анализу изворног кода више програмских језика"
#let autor = "Милан Лазаревић"

// На енглеском
#let naslov_eng = "RepoOptimizer - A distributed system for static analysis of source code in multiple programming languages"
#let autor_eng = "Milan Lazarević"

#let indeks = "SV4/2022"

// Име и презиме ментора
#let mentor = "Игор Дејановић"
// Звање: редовни професор, ванредни професор, доцент
#let mentor_zvanje = "редовни професор"

// Скинути коментаре са одговарајућих линија
#let studijski_program = "Софтверско инжењерство и информационе технологије"
//#let studijski_program = "Рачунарство и аутоматика"
// #let stepen = "Мастер академске студије"
#let stepen = "Основне академске студије"

#let godina = [#datetime.today().year()]

#let kljucne_reci = "статичка анализа кода, Rust, микросервиси, RabbitMQ, AST, Tree-sitter, детекција рањивости, Semgrep"
#let apstrakt = [
    Овај рад представља RepoOptimizer — дистрибуирани систем за статичку анализу кода за шест програмских језика. Реализован кроз Rust микросервисе и RabbitMQ, систем користи Tree-sitter и Semgrep за аутоматску детекцију, рангирање и предлагање исправки за безбедносне пропусте, лоше праксе, дуплирање и перформансне проблеме. Функционалности су доступне преко Web и CLI интерфејса, а рад обухвата опис архитектуре, тестирање и правце даљег развоја.
]

// На енглеском
#let kljucne_reci_eng = "static code analysis, Rust, microservices, RabbitMQ, AST, Tree-sitter, vulnerability detection, Semgrep"
#let apstrakt_eng = [
     This paper presents RepoOptimizer, a distributed static code analysis system for six programming languages. Implemented via Rust microservices and RabbitMQ, it leverages Tree-sitter and Semgrep to automatically detect, rank, and suggest fixes for security vulnerabilities, code smells, duplication, and performance issues. Accessible through Web and CLI interfaces, the paper outlines the system's architecture, testing approach, and future development directions.
]

// TODO: Текст задатка добијате од ментора. Заменити доле #lorem(100) са текстом задатка.
#let zadatak = [

Пројектовати и имплементирати _RepoOptimizer_, дистрибуирани систем за
аутоматизовану статичку анализу изворног кода више програмских језика (_Python_,
_JavaScript_, _Rust_, _Java_, _Go_). Архитектуру система засновати на скупу независних
микросервиса написаних у програмском језику _Rust_, међусобно повезаних асинхроном
разменом порука путем посредника _RabbitMQ_, уз примену хибридног модела
перзистенције (_PostgreSQL_, _MongoDB_ и _Redis_). Омогућити парсирање изворног кода
помоћу _Tree-sitter_ библиотеке, детекцију грешака из области безбедности,
перформанси, лоших пракси и дуплирања кода уз интеграцију алата _Semgrep_, као и
детерминистичко рангирање озбиљности налаза и шаблонско генерисање конкретних
предлога исправки. Приступ систему реализовати кроз веб кориснички интерфејс
(_React_) са визуелним приказом разлика у коду, као и кроз самостални
команднолинијски (_CLI_) алат погодан за интеграцију у _CI/CD_ окружења. Исправност
и поузданост решења потврдити јединичним тестовима пословне логике и практичном
демонстрацијом рада система над реалним примерима програмског кода.

При изради користити препоручену праксу из области софтверског инжењерства.
Детаљно документовати решење.

]

// TODO: Датум одбране и чланове комисије добијате од ментора
#let datum_odbrane = "10.09.2026"
#let komisija_predsednik = "Гордана Милосављевић"
#let komisija_predsednik_zvanje = "редовни професор"
#let komisija_clan = "Никола Лубурић"
#let komisija_clan_zvanje = "ванредни професор"

// На енглеском уписати чланове на латиници
#let komisija_predsednik_eng = "Gordana Milosavljević"
#let komisija_clan_eng = "Nikola Luburić"
#let mentor_eng = "Igor Dejanović"


// Ово даље углавном не треба мењати.

#let zvanje_eng = (
     "редовни професор": "full professor",
     "ванредни професор": "assoc. professor",
     "доцент": "asist. professor",
)
#let komisija_predsednik_zvanje_eng = zvanje_eng.at(komisija_predsednik_zvanje)
#let komisija_clan_zvanje_eng = zvanje_eng.at(komisija_clan_zvanje)
#let mentor_zvanje_eng = zvanje_eng.at(mentor_zvanje)


#let vrsta_rada = if stepen == "Мастер академске студије" {
    "Дипломски - мастер рад"
} else {
    "Дипломски - бечелор рад"
}

#let oblast = "Електротехничко и рачунарско инжењерство"
#let oblast_eng = "Electrical and Computer Engineering"
#let disciplina = "Примењене рачунарске науке и информатика"
#let disciplina_eng = "Applied computer science and informatics"

#import "funkcije.typ": *
// Поглавља/страна/цитата/табела/слика/графика/прилога
#let fizicki_opis = physical()
