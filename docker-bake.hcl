# docker-bake.hcl
# Kontrolise Docker Bake build paralelizam na nacin koji nije deprecated.
# `docker compose build` automatski koristi ovaj fajl ako je prisutan.
#
# max = 1 znaci da se servisi grade SEKVENCIJALNO (jedan po jedan), sto
# sprecava OOM na masinama sa manje RAM-a (6 Rust cargo build --release
# procesa paralelno = "cannot allocate memory" / SIGKILL).
#
# Kad Docker potpuno ukloni podrsku za COMPOSE_BAKE=false (sledeca major
# verzija Docker Desktop-a), ovaj fajl preuzima ulogu jedine kontrole.

group "default" {
  targets = [
    "auth_service",
    "parser_service",
    "analysis_service",
    "ml_ranker_service",
    "suggestion_generator_service",
    "api_gateway",
  ]
}

# Globalna opcija koja ogranicava koliko targeta se gradi paralelno.
# max = 1 => sekvencijalno. max = 2 => max 2 u isto vreme (kompromis
# izmedju brzine i memorije ako imas dovoljno RAM-a).
variable "MAX_PARALLEL" {
  default = "1"
}

target "auth_service" {
  context    = "."
  dockerfile = "Dockerfile.service"
  args = {
    SERVICE_NAME = "auth_service"
  }
  shm-size = "256m"
}

target "parser_service" {
  context    = "."
  dockerfile = "Dockerfile.service"
  args = {
    SERVICE_NAME = "parser_service"
  }
  shm-size = "256m"
}

target "analysis_service" {
  context    = "."
  dockerfile = "Dockerfile.analysis_service"
  shm-size   = "256m"
}

target "ml_ranker_service" {
  context    = "."
  dockerfile = "Dockerfile.service"
  args = {
    SERVICE_NAME = "ml_ranker_service"
  }
  shm-size = "256m"
}

target "suggestion_generator_service" {
  context    = "."
  dockerfile = "Dockerfile.service"
  args = {
    SERVICE_NAME = "suggestion_generator_service"
  }
  shm-size = "256m"
}

target "api_gateway" {
  context    = "."
  dockerfile = "Dockerfile.service"
  args = {
    SERVICE_NAME = "api_gateway"
  }
  shm-size = "256m"
}
