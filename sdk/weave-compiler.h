#ifndef WEAVE_COMPILER_SDK_H
#define WEAVE_COMPILER_SDK_H
#include <stdint.h>
/* ABI 1: opaque positive handles; no caller pointers. All failures are negative.
 * input_write appends one/two low little-endian bytes (word <= 65535).
 * Successful compile consumes input and returns an owned JSON output handle.
 * Failed preflight retains input. Every other retained handle must be dropped.
 * output_read returns up to two bytes; output_len determines the final byte count.
 * Instances/libraries contain at most 16 handles, 64 MiB buffer capacity, one compile.
 */
#ifdef __cplusplus
extern "C" {
#endif
#define WEAVE_COMPILER_INVALID_HANDLE (-1)
#define WEAVE_COMPILER_BOUNDS (-2)
#define WEAVE_COMPILER_BUDGET (-3)
#define WEAVE_COMPILER_INCOMPLETE (-4)
#define WEAVE_COMPILER_BUSY (-5)
#define WEAVE_COMPILER_INTERNAL (-6)
#define WEAVE_COMPILER_HANDLE_EXHAUSTED (-7)
int32_t weave_compiler_abi_version(void);
int32_t weave_compiler_input_new(uint32_t length);
int32_t weave_compiler_input_write(uint32_t handle, uint32_t word, uint32_t count);
int32_t weave_compiler_compile(uint32_t input);
int32_t weave_compiler_output_len(uint32_t handle);
int32_t weave_compiler_output_kind(uint32_t handle);
int32_t weave_compiler_output_read(uint32_t handle, uint32_t offset);
int32_t weave_compiler_drop(uint32_t handle);
#ifdef __cplusplus
}
#endif
#endif
