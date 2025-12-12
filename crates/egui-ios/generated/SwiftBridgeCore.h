#include <stdint.h>
#include <stdbool.h> 
typedef struct RustStr { uint8_t* const start; uintptr_t len; } RustStr;
typedef struct __private__FfiSlice { void* const start; uintptr_t len; } __private__FfiSlice;
void* __swift_bridge__null_pointer(void);


typedef struct __private__OptionU8 { uint8_t val; bool is_some; } __private__OptionU8;
typedef struct __private__OptionI8 { int8_t val; bool is_some; } __private__OptionI8;
typedef struct __private__OptionU16 { uint16_t val; bool is_some; } __private__OptionU16;
typedef struct __private__OptionI16 { int16_t val; bool is_some; } __private__OptionI16;
typedef struct __private__OptionU32 { uint32_t val; bool is_some; } __private__OptionU32;
typedef struct __private__OptionI32 { int32_t val; bool is_some; } __private__OptionI32;
typedef struct __private__OptionU64 { uint64_t val; bool is_some; } __private__OptionU64;
typedef struct __private__OptionI64 { int64_t val; bool is_some; } __private__OptionI64;
typedef struct __private__OptionUsize { uintptr_t val; bool is_some; } __private__OptionUsize;
typedef struct __private__OptionIsize { intptr_t val; bool is_some; } __private__OptionIsize;
typedef struct __private__OptionF32 { float val; bool is_some; } __private__OptionF32;
typedef struct __private__OptionF64 { double val; bool is_some; } __private__OptionF64;
typedef struct __private__OptionBool { bool val; bool is_some; } __private__OptionBool;

void* __swift_bridge__$Vec_u8$new();
void __swift_bridge__$Vec_u8$_free(void* const vec);
uintptr_t __swift_bridge__$Vec_u8$len(void* const vec);
void __swift_bridge__$Vec_u8$push(void* const vec, uint8_t val);
__private__OptionU8 __swift_bridge__$Vec_u8$pop(void* const vec);
__private__OptionU8 __swift_bridge__$Vec_u8$get(void* const vec, uintptr_t index);
__private__OptionU8 __swift_bridge__$Vec_u8$get_mut(void* const vec, uintptr_t index);
uint8_t const * __swift_bridge__$Vec_u8$as_ptr(void* const vec);

void* __swift_bridge__$Vec_u16$new();
void __swift_bridge__$Vec_u16$_free(void* const vec);
uintptr_t __swift_bridge__$Vec_u16$len(void* const vec);
void __swift_bridge__$Vec_u16$push(void* const vec, uint16_t val);
__private__OptionU16 __swift_bridge__$Vec_u16$pop(void* const vec);
__private__OptionU16 __swift_bridge__$Vec_u16$get(void* const vec, uintptr_t index);
__private__OptionU16 __swift_bridge__$Vec_u16$get_mut(void* const vec, uintptr_t index);
uint16_t const * __swift_bridge__$Vec_u16$as_ptr(void* const vec);

void* __swift_bridge__$Vec_u32$new();
void __swift_bridge__$Vec_u32$_free(void* const vec);
uintptr_t __swift_bridge__$Vec_u32$len(void* const vec);
void __swift_bridge__$Vec_u32$push(void* const vec, uint32_t val);
__private__OptionU32 __swift_bridge__$Vec_u32$pop(void* const vec);
__private__OptionU32 __swift_bridge__$Vec_u32$get(void* const vec, uintptr_t index);
__private__OptionU32 __swift_bridge__$Vec_u32$get_mut(void* const vec, uintptr_t index);
uint32_t const * __swift_bridge__$Vec_u32$as_ptr(void* const vec);

void* __swift_bridge__$Vec_u64$new();
void __swift_bridge__$Vec_u64$_free(void* const vec);
uintptr_t __swift_bridge__$Vec_u64$len(void* const vec);
void __swift_bridge__$Vec_u64$push(void* const vec, uint64_t val);
__private__OptionU64 __swift_bridge__$Vec_u64$pop(void* const vec);
__private__OptionU64 __swift_bridge__$Vec_u64$get(void* const vec, uintptr_t index);
__private__OptionU64 __swift_bridge__$Vec_u64$get_mut(void* const vec, uintptr_t index);
uint64_t const * __swift_bridge__$Vec_u64$as_ptr(void* const vec);

void* __swift_bridge__$Vec_usize$new();
void __swift_bridge__$Vec_usize$_free(void* const vec);
uintptr_t __swift_bridge__$Vec_usize$len(void* const vec);
void __swift_bridge__$Vec_usize$push(void* const vec, uintptr_t val);
__private__OptionUsize __swift_bridge__$Vec_usize$pop(void* const vec);
__private__OptionUsize __swift_bridge__$Vec_usize$get(void* const vec, uintptr_t index);
__private__OptionUsize __swift_bridge__$Vec_usize$get_mut(void* const vec, uintptr_t index);
uintptr_t const * __swift_bridge__$Vec_usize$as_ptr(void* const vec);

void* __swift_bridge__$Vec_i8$new();
void __swift_bridge__$Vec_i8$_free(void* const vec);
uintptr_t __swift_bridge__$Vec_i8$len(void* const vec);
void __swift_bridge__$Vec_i8$push(void* const vec, int8_t val);
__private__OptionI8 __swift_bridge__$Vec_i8$pop(void* const vec);
__private__OptionI8 __swift_bridge__$Vec_i8$get(void* const vec, uintptr_t index);
__private__OptionI8 __swift_bridge__$Vec_i8$get_mut(void* const vec, uintptr_t index);
int8_t const * __swift_bridge__$Vec_i8$as_ptr(void* const vec);

void* __swift_bridge__$Vec_i16$new();
void __swift_bridge__$Vec_i16$_free(void* const vec);
uintptr_t __swift_bridge__$Vec_i16$len(void* const vec);
void __swift_bridge__$Vec_i16$push(void* const vec, int16_t val);
__private__OptionI16 __swift_bridge__$Vec_i16$pop(void* const vec);
__private__OptionI16 __swift_bridge__$Vec_i16$get(void* const vec, uintptr_t index);
__private__OptionI16 __swift_bridge__$Vec_i16$get_mut(void* const vec, uintptr_t index);
int16_t const * __swift_bridge__$Vec_i16$as_ptr(void* const vec);

void* __swift_bridge__$Vec_i32$new();
void __swift_bridge__$Vec_i32$_free(void* const vec);
uintptr_t __swift_bridge__$Vec_i32$len(void* const vec);
void __swift_bridge__$Vec_i32$push(void* const vec, int32_t val);
__private__OptionI32 __swift_bridge__$Vec_i32$pop(void* const vec);
__private__OptionI32 __swift_bridge__$Vec_i32$get(void* const vec, uintptr_t index);
__private__OptionI32 __swift_bridge__$Vec_i32$get_mut(void* const vec, uintptr_t index);
int32_t const * __swift_bridge__$Vec_i32$as_ptr(void* const vec);

void* __swift_bridge__$Vec_i64$new();
void __swift_bridge__$Vec_i64$_free(void* const vec);
uintptr_t __swift_bridge__$Vec_i64$len(void* const vec);
void __swift_bridge__$Vec_i64$push(void* const vec, int64_t val);
__private__OptionI64 __swift_bridge__$Vec_i64$pop(void* const vec);
__private__OptionI64 __swift_bridge__$Vec_i64$get(void* const vec, uintptr_t index);
__private__OptionI64 __swift_bridge__$Vec_i64$get_mut(void* const vec, uintptr_t index);
int64_t const * __swift_bridge__$Vec_i64$as_ptr(void* const vec);

void* __swift_bridge__$Vec_isize$new();
void __swift_bridge__$Vec_isize$_free(void* const vec);
uintptr_t __swift_bridge__$Vec_isize$len(void* const vec);
void __swift_bridge__$Vec_isize$push(void* const vec, intptr_t val);
__private__OptionIsize __swift_bridge__$Vec_isize$pop(void* const vec);
__private__OptionIsize __swift_bridge__$Vec_isize$get(void* const vec, uintptr_t index);
__private__OptionIsize __swift_bridge__$Vec_isize$get_mut(void* const vec, uintptr_t index);
intptr_t const * __swift_bridge__$Vec_isize$as_ptr(void* const vec);

void* __swift_bridge__$Vec_bool$new();
void __swift_bridge__$Vec_bool$_free(void* const vec);
uintptr_t __swift_bridge__$Vec_bool$len(void* const vec);
void __swift_bridge__$Vec_bool$push(void* const vec, bool val);
__private__OptionBool __swift_bridge__$Vec_bool$pop(void* const vec);
__private__OptionBool __swift_bridge__$Vec_bool$get(void* const vec, uintptr_t index);
__private__OptionBool __swift_bridge__$Vec_bool$get_mut(void* const vec, uintptr_t index);
bool const * __swift_bridge__$Vec_bool$as_ptr(void* const vec);

void* __swift_bridge__$Vec_f32$new();
void __swift_bridge__$Vec_f32$_free(void* const vec);
uintptr_t __swift_bridge__$Vec_f32$len(void* const vec);
void __swift_bridge__$Vec_f32$push(void* const vec, float val);
__private__OptionF32 __swift_bridge__$Vec_f32$pop(void* const vec);
__private__OptionF32 __swift_bridge__$Vec_f32$get(void* const vec, uintptr_t index);
__private__OptionF32 __swift_bridge__$Vec_f32$get_mut(void* const vec, uintptr_t index);
float const * __swift_bridge__$Vec_f32$as_ptr(void* const vec);

void* __swift_bridge__$Vec_f64$new();
void __swift_bridge__$Vec_f64$_free(void* const vec);
uintptr_t __swift_bridge__$Vec_f64$len(void* const vec);
void __swift_bridge__$Vec_f64$push(void* const vec, double val);
__private__OptionF64 __swift_bridge__$Vec_f64$pop(void* const vec);
__private__OptionF64 __swift_bridge__$Vec_f64$get(void* const vec, uintptr_t index);
__private__OptionF64 __swift_bridge__$Vec_f64$get_mut(void* const vec, uintptr_t index);
double const * __swift_bridge__$Vec_f64$as_ptr(void* const vec);

#include <stdint.h>
typedef struct RustString RustString;
void __swift_bridge__$RustString$_free(void* self);

void* __swift_bridge__$Vec_RustString$new(void);
void __swift_bridge__$Vec_RustString$drop(void* vec_ptr);
void __swift_bridge__$Vec_RustString$push(void* vec_ptr, void* item_ptr);
void* __swift_bridge__$Vec_RustString$pop(void* vec_ptr);
void* __swift_bridge__$Vec_RustString$get(void* vec_ptr, uintptr_t index);
void* __swift_bridge__$Vec_RustString$get_mut(void* vec_ptr, uintptr_t index);
uintptr_t __swift_bridge__$Vec_RustString$len(void* vec_ptr);
void* __swift_bridge__$Vec_RustString$as_ptr(void* vec_ptr);

void* __swift_bridge__$RustString$new(void);
void* __swift_bridge__$RustString$new_with_str(struct RustStr str);
uintptr_t __swift_bridge__$RustString$len(void* self);
struct RustStr __swift_bridge__$RustString$as_str(void* self);
struct RustStr __swift_bridge__$RustString$trim(void* self);
bool __swift_bridge__$RustStr$partial_eq(struct RustStr lhs, struct RustStr rhs);


void __swift_bridge__$call_boxed_fn_once_no_args_no_return(void* boxed_fnonce);
void __swift_bridge__$free_boxed_fn_once_no_args_no_return(void* boxed_fnonce);


struct __private__ResultPtrAndPtr { bool is_ok; void* ok_or_err; };
