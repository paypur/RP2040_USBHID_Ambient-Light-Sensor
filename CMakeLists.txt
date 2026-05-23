cmake_minimum_required(VERSION 3.13)

# Include build functions from Pico SDK
include($ENV{PICO_SDK_PATH}/external/pico_sdk_import.cmake)

# Set name of project (as PROJECT_NAME) and C/C++ standards
project(als_hid_sensor C CXX ASM)
set(CMAKE_C_STANDARD 11)
set(CMAKE_CXX_STANDARD 17)

# Creates a pico-sdk subdirectory in our project for the libraries
pico_sdk_init()

include(FetchContent)
FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.6
)
FetchContent_MakeAvailable(Corrosion)

corrosion_import_crate(MANIFEST_PATH als_rust_lib/Cargo.toml)

# Tell CMake where to find the executable source files
add_executable(${PROJECT_NAME}
    main.c
    usb_descriptors.c
        rust_bindings.h
)

# Add the current directory to include path for tusb_config.h
target_include_directories(${PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

# Create map/bin/hex/uf2 files
pico_add_extra_outputs(${PROJECT_NAME})

# Link to pico_stdlib (gpio, time, etc. functions)
target_link_libraries(${PROJECT_NAME}
    PRIVATE
    pico_stdlib
    tinyusb_device
    tinyusb_board
    hardware_adc
    hardware_timer
    pico_unique_id
    als_rust_lib
)

# Enable usb output, disable uart output
pico_enable_stdio_usb(${PROJECT_NAME} 0)
pico_enable_stdio_uart(${PROJECT_NAME} 1)