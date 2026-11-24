cmake_minimum_required(VERSION 3.21)

project(app-boilerplate-rust LANGUAGES NONE)

set(SUPPORTED_DEVICES nanox nanosplus stax flex apex_p)

if(NOT DEVICE)
    message(FATAL_ERROR
        "DEVICE is not set. Pass -DDEVICE=<device> or use a CMake preset, "
        "e.g. `cmake --preset flex`. Supported devices: ${SUPPORTED_DEVICES}")
endif()

if(NOT DEVICE IN_LIST SUPPORTED_DEVICES)
    message(FATAL_ERROR "Unsupported DEVICE '${DEVICE}'. Supported devices: ${SUPPORTED_DEVICES}")
endif()

find_program(CARGO_EXECUTABLE cargo REQUIRED)

set(CARGO_LEDGER_FEATURES "" CACHE STRING "Comma-separated cargo features forwarded as --features to `cargo ledger build`")
set(CARGO_LEDGER_EXTRA_ARGS "" CACHE STRING "Extra arguments forwarded verbatim to `cargo ledger build`")

set(_cargo_ledger_build_command ${CARGO_EXECUTABLE} ledger build ${DEVICE})

if(CARGO_LEDGER_FEATURES)
    list(APPEND _cargo_ledger_build_command --features ${CARGO_LEDGER_FEATURES})
endif()

if(CARGO_LEDGER_EXTRA_ARGS)
    separate_arguments(_cargo_ledger_extra_args_list UNIX_COMMAND "${CARGO_LEDGER_EXTRA_ARGS}")
    list(APPEND _cargo_ledger_build_command ${_cargo_ledger_extra_args_list})
endif()

add_custom_target(app ALL
    COMMAND ${_cargo_ledger_build_command}
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
    USES_TERMINAL
    COMMENT "cargo ledger build ${DEVICE}"
    VERBATIM
)
