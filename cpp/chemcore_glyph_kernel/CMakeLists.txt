set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

add_library(
  chemcore_glyph_kernel
  STATIC
  src/glyph_kernel.cpp
  src/glyph_kernel_c_api.cpp
)

target_include_directories(
  chemcore_glyph_kernel
  PUBLIC
  ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_compile_features(chemcore_glyph_kernel PUBLIC cxx_std_17)

add_library(chemcore::glyph_kernel ALIAS chemcore_glyph_kernel)

if(EMSCRIPTEN)
  add_executable(
    chemcore_glyph_kernel_wasm
    src/glyph_kernel.cpp
    src/glyph_kernel_c_api.cpp
  )

  target_include_directories(
    chemcore_glyph_kernel_wasm
    PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/include
  )

  set_target_properties(
    chemcore_glyph_kernel_wasm
    PROPERTIES
    OUTPUT_NAME chemcore_glyph_kernel
    RUNTIME_OUTPUT_DIRECTORY ${CMAKE_SOURCE_DIR}/viewer
    SUFFIX ".js"
  )

  target_link_options(
    chemcore_glyph_kernel_wasm
    PRIVATE
    "--no-entry"
    "-sMODULARIZE=1"
    "-sEXPORT_ES6=1"
    "-sEXPORT_NAME=createChemcoreGlyphKernelModule"
    "-sENVIRONMENT=web"
    "-sALLOW_MEMORY_GROWTH=1"
    "-sEXPORTED_FUNCTIONS=['_malloc','_free','_chemcore_layout_glyph_run_aligned']"
    "-sEXPORTED_RUNTIME_METHODS=['HEAPU8','HEAPU32','HEAP32','HEAPF32']"
  )
endif()

add_executable(
  chemcore_glyph_demo
  tools/chemcore_glyph_demo.cpp
)

target_link_libraries(chemcore_glyph_demo PRIVATE chemcore::glyph_kernel)

add_executable(
  chemcore_glyph_svg_demo
  tools/chemcore_glyph_svg_demo.cpp
)

target_link_libraries(chemcore_glyph_svg_demo PRIVATE chemcore::glyph_kernel)

add_executable(
  chemcore_glyph_kernel_smoke
  tests/glyph_kernel_smoke.cpp
)

target_link_libraries(chemcore_glyph_kernel_smoke PRIVATE chemcore::glyph_kernel)

add_test(
  NAME chemcore_glyph_kernel_smoke
  COMMAND chemcore_glyph_kernel_smoke
)

find_program(CHEMCORE_PYTHON3_EXECUTABLE NAMES python3 python)

if(CHEMCORE_PYTHON3_EXECUTABLE)
  execute_process(
    COMMAND
      ${CHEMCORE_PYTHON3_EXECUTABLE}
      ${CMAKE_SOURCE_DIR}/scripts/glyph_kernel_reference.py
      dump
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
    OUTPUT_QUIET
    ERROR_QUIET
    RESULT_VARIABLE CHEMCORE_GLYPH_REFERENCE_READY
  )

  if(CHEMCORE_GLYPH_REFERENCE_READY EQUAL 0)
    set(CHEMCORE_GLYPH_PREVIEW_SVG ${CMAKE_CURRENT_BINARY_DIR}/chemcore_glyph_kernel_preview.svg)

    add_test(
      NAME chemcore_glyph_svg_demo
      COMMAND chemcore_glyph_svg_demo -o ${CHEMCORE_GLYPH_PREVIEW_SVG}
    )

    add_test(
      NAME chemcore_glyph_reference_check
      COMMAND
        ${CHEMCORE_PYTHON3_EXECUTABLE}
        ${CMAKE_SOURCE_DIR}/scripts/glyph_kernel_reference.py
        check
        --svg
        ${CHEMCORE_GLYPH_PREVIEW_SVG}
    )

    set_tests_properties(
      chemcore_glyph_reference_check
      PROPERTIES
      DEPENDS chemcore_glyph_svg_demo
    )
  endif()
endif()
