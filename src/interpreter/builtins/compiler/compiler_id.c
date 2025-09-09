#if defined(__EMSCRIPTEN__)
#define MESON_COMPILER_FAMILY emscripten
#elif defined(__clang__)
#define MESON_COMPILER_FAMILY clang
#elif defined(__GNUC__)
#define MESON_COMPILER_FAMILY gcc
#elif defined(_MSC_VER)
#define MESON_COMPILER_FAMILY msvc
#endif
"MESON_DELIMITER" MESON_COMPILER_FAMILY