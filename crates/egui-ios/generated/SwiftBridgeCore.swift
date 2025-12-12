import Foundation

extension RustString {
    public func toString() -> String {
        let str = self.as_str()
        let string = str.toString()

        return string
    }
}

extension RustStr {
    func toBufferPointer() -> UnsafeBufferPointer<UInt8> {
        let bytes = UnsafeBufferPointer(start: self.start, count: Int(self.len))
        return bytes
    }

    public func toString() -> String {
        let bytes = self.toBufferPointer()
        return String(bytes: bytes, encoding: .utf8)!
    }
}
extension RustStr: Identifiable {
    public var id: String {
        self.toString()
    }
}
extension RustStr: Equatable {
    public static func == (lhs: RustStr, rhs: RustStr) -> Bool {
        return __swift_bridge__$RustStr$partial_eq(lhs, rhs);
    }
}

public protocol IntoRustString {
    func intoRustString() -> RustString;
}

extension String: IntoRustString {
    public func intoRustString() -> RustString {
        // TODO: When passing an owned Swift std String to Rust we've being wasteful here in that
        //  we're creating a RustString (which involves Boxing a Rust std::string::String)
        //  only to unbox it back into a String once it gets to the Rust side.
        //
        //  A better approach would be to pass a RustStr to the Rust side and then have Rust
        //  call `.to_string()` on the RustStr.
        RustString(self)
    }
}

extension RustString: IntoRustString {
    public func intoRustString() -> RustString {
        self
    }
}

/// If the String is Some:
///   Safely get a scoped pointer to the String and then call the callback with a RustStr
///   that uses that pointer.
///
/// If the String is None:
///   Call the callback with a RustStr that has a null pointer.
///   The Rust side will know to treat this as `None`.
func optionalStringIntoRustString<S: IntoRustString>(_ string: Optional<S>) -> RustString? {
    if let val = string {
        return val.intoRustString()
    } else {
        return nil
    }
}

/// Used to safely get a pointer to a sequence of utf8 bytes, represented as a `RustStr`.
///
/// For example, the Swift `String` implementation of the `ToRustStr` protocol does the following:
/// 1. Use Swift's `String.utf8.withUnsafeBufferPointer` to get a pointer to the strings underlying
///    utf8 bytes.
/// 2. Construct a `RustStr` that points to these utf8 bytes. This is safe because `withUnsafeBufferPointer`
///    guarantees that the buffer pointer will be valid for the duration of the `withUnsafeBufferPointer`
///    callback.
/// 3. Pass the `RustStr` to the closure that was passed into `RustStr.toRustStr`.
public protocol ToRustStr {
    func toRustStr<T> (_ withUnsafeRustStr: (RustStr) -> T) -> T;
}

extension String: ToRustStr {
    /// Safely get a scoped pointer to the String and then call the callback with a RustStr
    /// that uses that pointer.
    public func toRustStr<T> (_ withUnsafeRustStr: (RustStr) -> T) -> T {
        return self.utf8CString.withUnsafeBufferPointer({ bufferPtr in
            let rustStr = RustStr(
                start: UnsafeMutableRawPointer(mutating: bufferPtr.baseAddress!).assumingMemoryBound(to: UInt8.self),
                // Subtract 1 because of the null termination character at the end
                len: UInt(bufferPtr.count - 1)
            )
            return withUnsafeRustStr(rustStr)
        })
    }
}

extension RustStr: ToRustStr {
    public func toRustStr<T> (_ withUnsafeRustStr: (RustStr) -> T) -> T {
        return withUnsafeRustStr(self)
    }
}

func optionalRustStrToRustStr<S: ToRustStr, T>(_ str: Optional<S>, _ withUnsafeRustStr: (RustStr) -> T) -> T {
    if let val = str {
        return val.toRustStr(withUnsafeRustStr)
    } else {
        return withUnsafeRustStr(RustStr(start: nil, len: 0))
    }
}
public class RustVec<T: Vectorizable> {
    var ptr: UnsafeMutableRawPointer
    var isOwned: Bool = true

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }

    public init() {
        ptr = T.vecOfSelfNew()
        isOwned = true
    }

    public func push (value: T) {
        T.vecOfSelfPush(vecPtr: ptr, value: value)
    }

    public func pop () -> Optional<T> {
        T.vecOfSelfPop(vecPtr: ptr)
    }

    public func get(index: UInt) -> Optional<T.SelfRef> {
         T.vecOfSelfGet(vecPtr: ptr, index: index)
    }

    public func as_ptr() -> UnsafePointer<T.SelfRef> {
        UnsafePointer<T.SelfRef>(OpaquePointer(T.vecOfSelfAsPtr(vecPtr: ptr)))
    }

    /// Rust returns a UInt, but we cast to an Int because many Swift APIs such as
    /// `ForEach(0..rustVec.len())` expect Int.
    public func len() -> Int {
        Int(T.vecOfSelfLen(vecPtr: ptr))
    }

    deinit {
        if isOwned {
            T.vecOfSelfFree(vecPtr: ptr)
        }
    }
}

extension RustVec: Sequence {
    public func makeIterator() -> RustVecIterator<T> {
        return RustVecIterator(self)
    }
}

public struct RustVecIterator<T: Vectorizable>: IteratorProtocol {
    var rustVec: RustVec<T>
    var index: UInt = 0

    init (_ rustVec: RustVec<T>) {
        self.rustVec = rustVec
    }

    public mutating func next() -> T.SelfRef? {
        let val = rustVec.get(index: index)
        index += 1
        return val
    }
}

extension RustVec: Collection {
    public typealias Index = Int

    public func index(after i: Int) -> Int {
        i + 1
    }

    public subscript(position: Int) -> T.SelfRef {
        self.get(index: UInt(position))!
    }

    public var startIndex: Int {
        0
    }

    public var endIndex: Int {
        self.len()
    }
}

extension RustVec: RandomAccessCollection {}

extension UnsafeBufferPointer {
    func toFfiSlice () -> __private__FfiSlice {
        __private__FfiSlice(start: UnsafeMutablePointer(mutating: self.baseAddress), len: UInt(self.count))
    }
}

public protocol Vectorizable {
    associatedtype SelfRef
    associatedtype SelfRefMut

    static func vecOfSelfNew() -> UnsafeMutableRawPointer;

    static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer)

    static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self)

    static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self>

    static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<SelfRef>

    static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<SelfRefMut>

    static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<SelfRef>

    static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt
}

extension UInt8: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_u8$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_u8$_free(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self) {
        __swift_bridge__$Vec_u8$push(vecPtr, value)
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let val = __swift_bridge__$Vec_u8$pop(vecPtr)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_u8$get(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_u8$get_mut(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<Self> {
        UnsafePointer<Self>(OpaquePointer(__swift_bridge__$Vec_u8$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_u8$len(vecPtr)
    }
}
    
extension UInt16: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_u16$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_u16$_free(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self) {
        __swift_bridge__$Vec_u16$push(vecPtr, value)
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let val = __swift_bridge__$Vec_u16$pop(vecPtr)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_u16$get(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_u16$get_mut(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<Self> {
        UnsafePointer<Self>(OpaquePointer(__swift_bridge__$Vec_u16$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_u16$len(vecPtr)
    }
}
    
extension UInt32: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_u32$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_u32$_free(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self) {
        __swift_bridge__$Vec_u32$push(vecPtr, value)
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let val = __swift_bridge__$Vec_u32$pop(vecPtr)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_u32$get(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_u32$get_mut(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<Self> {
        UnsafePointer<Self>(OpaquePointer(__swift_bridge__$Vec_u32$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_u32$len(vecPtr)
    }
}
    
extension UInt64: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_u64$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_u64$_free(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self) {
        __swift_bridge__$Vec_u64$push(vecPtr, value)
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let val = __swift_bridge__$Vec_u64$pop(vecPtr)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_u64$get(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_u64$get_mut(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<Self> {
        UnsafePointer<Self>(OpaquePointer(__swift_bridge__$Vec_u64$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_u64$len(vecPtr)
    }
}
    
extension UInt: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_usize$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_usize$_free(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self) {
        __swift_bridge__$Vec_usize$push(vecPtr, value)
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let val = __swift_bridge__$Vec_usize$pop(vecPtr)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_usize$get(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_usize$get_mut(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<Self> {
        UnsafePointer<Self>(OpaquePointer(__swift_bridge__$Vec_usize$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_usize$len(vecPtr)
    }
}
    
extension Int8: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_i8$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_i8$_free(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self) {
        __swift_bridge__$Vec_i8$push(vecPtr, value)
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let val = __swift_bridge__$Vec_i8$pop(vecPtr)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_i8$get(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_i8$get_mut(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<Self> {
        UnsafePointer<Self>(OpaquePointer(__swift_bridge__$Vec_i8$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_i8$len(vecPtr)
    }
}
    
extension Int16: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_i16$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_i16$_free(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self) {
        __swift_bridge__$Vec_i16$push(vecPtr, value)
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let val = __swift_bridge__$Vec_i16$pop(vecPtr)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_i16$get(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_i16$get_mut(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<Self> {
        UnsafePointer<Self>(OpaquePointer(__swift_bridge__$Vec_i16$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_i16$len(vecPtr)
    }
}
    
extension Int32: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_i32$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_i32$_free(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self) {
        __swift_bridge__$Vec_i32$push(vecPtr, value)
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let val = __swift_bridge__$Vec_i32$pop(vecPtr)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_i32$get(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_i32$get_mut(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<Self> {
        UnsafePointer<Self>(OpaquePointer(__swift_bridge__$Vec_i32$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_i32$len(vecPtr)
    }
}
    
extension Int64: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_i64$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_i64$_free(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self) {
        __swift_bridge__$Vec_i64$push(vecPtr, value)
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let val = __swift_bridge__$Vec_i64$pop(vecPtr)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_i64$get(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_i64$get_mut(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<Self> {
        UnsafePointer<Self>(OpaquePointer(__swift_bridge__$Vec_i64$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_i64$len(vecPtr)
    }
}
    
extension Int: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_isize$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_isize$_free(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self) {
        __swift_bridge__$Vec_isize$push(vecPtr, value)
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let val = __swift_bridge__$Vec_isize$pop(vecPtr)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_isize$get(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_isize$get_mut(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<Self> {
        UnsafePointer<Self>(OpaquePointer(__swift_bridge__$Vec_isize$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_isize$len(vecPtr)
    }
}
    
extension Bool: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_bool$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_bool$_free(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self) {
        __swift_bridge__$Vec_bool$push(vecPtr, value)
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let val = __swift_bridge__$Vec_bool$pop(vecPtr)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_bool$get(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_bool$get_mut(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<Self> {
        UnsafePointer<Self>(OpaquePointer(__swift_bridge__$Vec_bool$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_bool$len(vecPtr)
    }
}
    
extension Float: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_f32$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_f32$_free(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self) {
        __swift_bridge__$Vec_f32$push(vecPtr, value)
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let val = __swift_bridge__$Vec_f32$pop(vecPtr)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_f32$get(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_f32$get_mut(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<Self> {
        UnsafePointer<Self>(OpaquePointer(__swift_bridge__$Vec_f32$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_f32$len(vecPtr)
    }
}
    
extension Double: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_f64$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_f64$_free(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self) {
        __swift_bridge__$Vec_f64$push(vecPtr, value)
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let val = __swift_bridge__$Vec_f64$pop(vecPtr)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_f64$get(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {
        let val = __swift_bridge__$Vec_f64$get_mut(vecPtr, index)
        if val.is_some {
            return val.val
        } else {
            return nil
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<Self> {
        UnsafePointer<Self>(OpaquePointer(__swift_bridge__$Vec_f64$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_f64$len(vecPtr)
    }
}
    
protocol SwiftBridgeGenericFreer {
    func rust_free();
}
    
protocol SwiftBridgeGenericCopyTypeFfiRepr {}

public class RustString: RustStringRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$RustString$_free(ptr)
        }
    }
}

/// Tested in:
///   SwiftRustIntegrationTestRunner/SwiftRustIntegrationTestRunnerTests/ResultTests.swift:
///  `func testSwiftCallRustReturnsResultString()`
extension RustString: Error {}

// THREAD SAFETY: `RustString`, `RustStringRef` and `RustStringRefMut` are safe to send across threads as long as the
// ownership and aliasing rules are followed.
// This is because the underlying Rust `std::string::String`, `&str` and `&mut str` are all `Send+Sync`.
// See the `Safety` chapter in the book for more information about memory and thread safety rules.
//
// For now we have implemented `Sendable` for `RustString`. If users need `RustStringRef` or `RustStringRefMut` to
// implement `Sendable` then we can implement those as well.
//
// Tested in:
//  `SwiftRustIntegrationTestRunner/SwiftRustIntegrationTestRunnerTests/SendableTests.swift`
//  `func testSendableRustString()`
extension RustString: @unchecked Sendable {}

extension RustString {
    public convenience init() {
        self.init(ptr: __swift_bridge__$RustString$new())
    }

    public convenience init<GenericToRustStr: ToRustStr>(_ str: GenericToRustStr) {
        self.init(ptr: str.toRustStr({ strAsRustStr in
            __swift_bridge__$RustString$new_with_str(strAsRustStr)
        }))
    }
}
public class RustStringRefMut: RustStringRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class RustStringRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension RustStringRef {
    public func len() -> UInt {
        __swift_bridge__$RustString$len(ptr)
    }

    public func as_str() -> RustStr {
        __swift_bridge__$RustString$as_str(ptr)
    }

    public func trim() -> RustStr {
        __swift_bridge__$RustString$trim(ptr)
    }
}
extension RustString: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_RustString$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_RustString$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: RustString) {
        __swift_bridge__$Vec_RustString$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_RustString$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (RustString(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RustStringRef> {
        let pointer = __swift_bridge__$Vec_RustString$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RustStringRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RustStringRefMut> {
        let pointer = __swift_bridge__$Vec_RustString$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RustStringRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<RustStringRef> {
        UnsafePointer<RustStringRef>(OpaquePointer(__swift_bridge__$Vec_RustString$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_RustString$len(vecPtr)
    }
}


public class __private__RustFnOnceCallbackNoArgsNoRet {
    var ptr: UnsafeMutableRawPointer
    var called = false

    init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }

    deinit {
        if !called {
            __swift_bridge__$free_boxed_fn_once_no_args_no_return(ptr)
        }
    }

    func call() {
        if called {
            fatalError("Cannot call a Rust FnOnce function twice")
        }
        called = true
        return __swift_bridge__$call_boxed_fn_once_no_args_no_return(ptr)
    }
}


public enum RustResult<T, E> {
    case Ok(T)
    case Err(E)
}

extension RustResult {
    func ok() -> T? {
        switch self {
        case .Ok(let ok):
            return ok
        case .Err(_):
            return nil
        }
    }

    func err() -> E? {
        switch self {
        case .Ok(_):
            return nil
        case .Err(let err):
            return err
        }
    }
    
    func toResult() -> Result<T, E>
    where E: Error {
        switch self {
        case .Ok(let ok):
            return .success(ok)
        case .Err(let err):
            return .failure(err)
        }
    }
}


extension __private__OptionU8 {
    func intoSwiftRepr() -> Optional<UInt8> {
        if self.is_some {
            return self.val 
        } else {
            return nil
        }
    }

    init(_ val: Optional<UInt8>) {
        if let val = val {
            self = Self(val: val, is_some: true) 
        } else {
            self = Self(val: 123, is_some: false) 
        }
    }
}
extension Optional where Wrapped == UInt8 {
    func intoFfiRepr() -> __private__OptionU8 {
        __private__OptionU8(self) 
    }
}

extension __private__OptionI8 {
    func intoSwiftRepr() -> Optional<Int8> {
        if self.is_some {
            return self.val 
        } else {
            return nil
        }
    }

    init(_ val: Optional<Int8>) {
        if let val = val {
            self = Self(val: val, is_some: true) 
        } else {
            self = Self(val: 123, is_some: false) 
        }
    }
}
extension Optional where Wrapped == Int8 {
    func intoFfiRepr() -> __private__OptionI8 {
        __private__OptionI8(self) 
    }
}

extension __private__OptionU16 {
    func intoSwiftRepr() -> Optional<UInt16> {
        if self.is_some {
            return self.val 
        } else {
            return nil
        }
    }

    init(_ val: Optional<UInt16>) {
        if let val = val {
            self = Self(val: val, is_some: true) 
        } else {
            self = Self(val: 123, is_some: false) 
        }
    }
}
extension Optional where Wrapped == UInt16 {
    func intoFfiRepr() -> __private__OptionU16 {
        __private__OptionU16(self) 
    }
}

extension __private__OptionI16 {
    func intoSwiftRepr() -> Optional<Int16> {
        if self.is_some {
            return self.val 
        } else {
            return nil
        }
    }

    init(_ val: Optional<Int16>) {
        if let val = val {
            self = Self(val: val, is_some: true) 
        } else {
            self = Self(val: 123, is_some: false) 
        }
    }
}
extension Optional where Wrapped == Int16 {
    func intoFfiRepr() -> __private__OptionI16 {
        __private__OptionI16(self) 
    }
}

extension __private__OptionU32 {
    func intoSwiftRepr() -> Optional<UInt32> {
        if self.is_some {
            return self.val 
        } else {
            return nil
        }
    }

    init(_ val: Optional<UInt32>) {
        if let val = val {
            self = Self(val: val, is_some: true) 
        } else {
            self = Self(val: 123, is_some: false) 
        }
    }
}
extension Optional where Wrapped == UInt32 {
    func intoFfiRepr() -> __private__OptionU32 {
        __private__OptionU32(self) 
    }
}

extension __private__OptionI32 {
    func intoSwiftRepr() -> Optional<Int32> {
        if self.is_some {
            return self.val 
        } else {
            return nil
        }
    }

    init(_ val: Optional<Int32>) {
        if let val = val {
            self = Self(val: val, is_some: true) 
        } else {
            self = Self(val: 123, is_some: false) 
        }
    }
}
extension Optional where Wrapped == Int32 {
    func intoFfiRepr() -> __private__OptionI32 {
        __private__OptionI32(self) 
    }
}

extension __private__OptionU64 {
    func intoSwiftRepr() -> Optional<UInt64> {
        if self.is_some {
            return self.val 
        } else {
            return nil
        }
    }

    init(_ val: Optional<UInt64>) {
        if let val = val {
            self = Self(val: val, is_some: true) 
        } else {
            self = Self(val: 123, is_some: false) 
        }
    }
}
extension Optional where Wrapped == UInt64 {
    func intoFfiRepr() -> __private__OptionU64 {
        __private__OptionU64(self) 
    }
}

extension __private__OptionI64 {
    func intoSwiftRepr() -> Optional<Int64> {
        if self.is_some {
            return self.val 
        } else {
            return nil
        }
    }

    init(_ val: Optional<Int64>) {
        if let val = val {
            self = Self(val: val, is_some: true) 
        } else {
            self = Self(val: 123, is_some: false) 
        }
    }
}
extension Optional where Wrapped == Int64 {
    func intoFfiRepr() -> __private__OptionI64 {
        __private__OptionI64(self) 
    }
}

extension __private__OptionUsize {
    func intoSwiftRepr() -> Optional<UInt> {
        if self.is_some {
            return self.val 
        } else {
            return nil
        }
    }

    init(_ val: Optional<UInt>) {
        if let val = val {
            self = Self(val: val, is_some: true) 
        } else {
            self = Self(val: 123, is_some: false) 
        }
    }
}
extension Optional where Wrapped == UInt {
    func intoFfiRepr() -> __private__OptionUsize {
        __private__OptionUsize(self) 
    }
}

extension __private__OptionIsize {
    func intoSwiftRepr() -> Optional<Int> {
        if self.is_some {
            return self.val 
        } else {
            return nil
        }
    }

    init(_ val: Optional<Int>) {
        if let val = val {
            self = Self(val: val, is_some: true) 
        } else {
            self = Self(val: 123, is_some: false) 
        }
    }
}
extension Optional where Wrapped == Int {
    func intoFfiRepr() -> __private__OptionIsize {
        __private__OptionIsize(self) 
    }
}

extension __private__OptionF32 {
    func intoSwiftRepr() -> Optional<Float> {
        if self.is_some {
            return self.val 
        } else {
            return nil
        }
    }

    init(_ val: Optional<Float>) {
        if let val = val {
            self = Self(val: val, is_some: true) 
        } else {
            self = Self(val: 123.4, is_some: false) 
        }
    }
}
extension Optional where Wrapped == Float {
    func intoFfiRepr() -> __private__OptionF32 {
        __private__OptionF32(self) 
    }
}

extension __private__OptionF64 {
    func intoSwiftRepr() -> Optional<Double> {
        if self.is_some {
            return self.val 
        } else {
            return nil
        }
    }

    init(_ val: Optional<Double>) {
        if let val = val {
            self = Self(val: val, is_some: true) 
        } else {
            self = Self(val: 123.4, is_some: false) 
        }
    }
}
extension Optional where Wrapped == Double {
    func intoFfiRepr() -> __private__OptionF64 {
        __private__OptionF64(self) 
    }
}

extension __private__OptionBool {
    func intoSwiftRepr() -> Optional<Bool> {
        if self.is_some {
            return self.val 
        } else {
            return nil
        }
    }

    init(_ val: Optional<Bool>) {
        if let val = val {
            self = Self(val: val, is_some: true) 
        } else {
            self = Self(val: false, is_some: false) 
        }
    }
}
extension Optional where Wrapped == Bool {
    func intoFfiRepr() -> __private__OptionBool {
        __private__OptionBool(self) 
    }
}
