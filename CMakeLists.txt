# SPDX-FileCopyrightText: © Matteo Settenvini <matteo.settenvini@montecristosoftware.eu>
# SPDX-License-Identifier: EUPL-1.2

cmake_minimum_required(VERSION 3.25)

project(serves3 VERSION 1.0.0 LANGUAGES NONE)

include(FetchContent)
FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.4.1
)
FetchContent_MakeAvailable(Corrosion)

corrosion_import_crate(
  MANIFEST_PATH Cargo.toml
  IMPORTED_CRATES imported_crates
  LOCKED)
message(STATUS "Imported crates: ${imported_crates}")

install(IMPORTED_RUNTIME_ARTIFACTS serves3)
