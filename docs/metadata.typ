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

#let kljucne_reci = "Шаблон, завршни рад, упутство"
#let apstrakt = [
    Овај рад представља RepoOptimizer — дистрибуирани систем за статичку анализу кода за шест програмских језика. Реализован кроз Rust микросервисе и RabbitMQ, систем користи Tree-sitter и Semgrep за аутоматску детекцију, рангирање и предлагање исправки за безбедносне пропусте, лоше праксе, дуплирање и перформансне проблеме. Функционалности су доступне преко Web и CLI интерфејса, а рад обухвата опис архитектуре, тестирање и правце даљег развоја.
]

// На енглеском
#let kljucne_reci_eng = "Template, thesis, tutorial"
#let apstrakt_eng = [
     This paper presents RepoOptimizer, a distributed static code analysis system for six programming languages. Implemented via Rust microservices and RabbitMQ, it leverages Tree-sitter and Semgrep to automatically detect, rank, and suggest fixes for security vulnerabilities, code smells, duplication, and performance issues. Accessible through Web and CLI interfaces, the paper outlines the system's architecture, testing approach, and future development directions.
]

// TODO: Текст задатка добијате од ментора. Заменити доле #lorem(100) са текстом задатка.
#let zadatak = [
     #lorem(100)
]

// TODO: Датум одбране и чланове комисије добијате од ментора
#let datum_odbrane = "01.01.2025"
#let komisija_predsednik = "Петар Петровић"
#let komisija_predsednik_zvanje = "ванредни професор"
#let komisija_clan = "Марко Марковић"
#let komisija_clan_zvanje = "доцент"

// На енглеском уписати чланове на латиници
#let komisija_predsednik_eng = "Petar Petrović"
#let komisija_clan_eng = "Marko Marković"
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
