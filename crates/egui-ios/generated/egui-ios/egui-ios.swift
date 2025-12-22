
public class InputEvent: InputEventRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$InputEvent$_free(ptr)
        }
    }
}
extension InputEvent {
    class public func from_pointer_moved(_ x: Float, _ y: Float) -> InputEvent {
        InputEvent(ptr: __swift_bridge__$InputEvent$from_pointer_moved(x, y))
    }

    class public func from_mouse_wheel(_ x: Float, _ y: Float) -> InputEvent {
        InputEvent(ptr: __swift_bridge__$InputEvent$from_mouse_wheel(x, y))
    }

    class public func from_left_mouse_down(_ x: Float, _ y: Float, _ pressed: Bool) -> InputEvent {
        InputEvent(ptr: __swift_bridge__$InputEvent$from_left_mouse_down(x, y, pressed))
    }

    class public func from_right_mouse_down(_ x: Float, _ y: Float, _ pressed: Bool) -> InputEvent {
        InputEvent(ptr: __swift_bridge__$InputEvent$from_right_mouse_down(x, y, pressed))
    }

    class public func from_window_focused(_ focused: Bool) -> InputEvent {
        InputEvent(ptr: __swift_bridge__$InputEvent$from_window_focused(focused))
    }

    class public func from_scene_phase_changed(_ phase: UInt8) -> InputEvent {
        InputEvent(ptr: __swift_bridge__$InputEvent$from_scene_phase_changed(phase))
    }

    class public func from_text_commit<GenericIntoRustString: IntoRustString>(_ text: GenericIntoRustString) -> InputEvent {
        InputEvent(ptr: __swift_bridge__$InputEvent$from_text_commit({ let rustString = text.intoRustString(); rustString.isOwned = false; return rustString.ptr }()))
    }

    class public func from_ime_preedit<GenericIntoRustString: IntoRustString>(_ text: GenericIntoRustString) -> InputEvent {
        InputEvent(ptr: __swift_bridge__$InputEvent$from_ime_preedit({ let rustString = text.intoRustString(); rustString.isOwned = false; return rustString.ptr }()))
    }

    class public func from_keyboard_visibility(_ visible: Bool) -> InputEvent {
        InputEvent(ptr: __swift_bridge__$InputEvent$from_keyboard_visibility(visible))
    }

    class public func from_virtual_key(_ key_code: UInt8, _ pressed: Bool) -> InputEvent {
        InputEvent(ptr: __swift_bridge__$InputEvent$from_virtual_key(key_code, pressed))
    }
}
public class InputEventRefMut: InputEventRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class InputEventRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension InputEvent: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_InputEvent$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_InputEvent$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: InputEvent) {
        __swift_bridge__$Vec_InputEvent$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_InputEvent$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (InputEvent(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<InputEventRef> {
        let pointer = __swift_bridge__$Vec_InputEvent$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return InputEventRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<InputEventRefMut> {
        let pointer = __swift_bridge__$Vec_InputEvent$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return InputEventRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<InputEventRef> {
        UnsafePointer<InputEventRef>(OpaquePointer(__swift_bridge__$Vec_InputEvent$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_InputEvent$len(vecPtr)
    }
}


public class OutputState: OutputStateRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$OutputState$_free(ptr)
        }
    }
}
public class OutputStateRefMut: OutputStateRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OutputStateRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension OutputStateRef {
    public func get_cursor_icon() -> CursorIconRef {
        CursorIconRef(ptr: __swift_bridge__$OutputState$get_cursor_icon(ptr))
    }

    public func wants_keyboard() -> Bool {
        __swift_bridge__$OutputState$wants_keyboard(ptr)
    }

    public func has_ime_rect() -> Bool {
        __swift_bridge__$OutputState$has_ime_rect(ptr)
    }

    public func get_ime_rect_x() -> Float {
        __swift_bridge__$OutputState$get_ime_rect_x(ptr)
    }

    public func get_ime_rect_y() -> Float {
        __swift_bridge__$OutputState$get_ime_rect_y(ptr)
    }

    public func get_ime_rect_width() -> Float {
        __swift_bridge__$OutputState$get_ime_rect_width(ptr)
    }

    public func get_ime_rect_height() -> Float {
        __swift_bridge__$OutputState$get_ime_rect_height(ptr)
    }
}
extension OutputState: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_OutputState$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_OutputState$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: OutputState) {
        __swift_bridge__$Vec_OutputState$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_OutputState$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (OutputState(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OutputStateRef> {
        let pointer = __swift_bridge__$Vec_OutputState$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OutputStateRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OutputStateRefMut> {
        let pointer = __swift_bridge__$Vec_OutputState$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OutputStateRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OutputStateRef> {
        UnsafePointer<OutputStateRef>(OpaquePointer(__swift_bridge__$Vec_OutputState$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_OutputState$len(vecPtr)
    }
}


public class CursorIcon: CursorIconRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$CursorIcon$_free(ptr)
        }
    }
}
public class CursorIconRefMut: CursorIconRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class CursorIconRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension CursorIconRef {
    public func is_default() -> Bool {
        __swift_bridge__$CursorIcon$is_default(ptr)
    }

    public func is_pointing_hand() -> Bool {
        __swift_bridge__$CursorIcon$is_pointing_hand(ptr)
    }

    public func is_resize_horizontal() -> Bool {
        __swift_bridge__$CursorIcon$is_resize_horizontal(ptr)
    }

    public func is_resize_vertical() -> Bool {
        __swift_bridge__$CursorIcon$is_resize_vertical(ptr)
    }

    public func is_text() -> Bool {
        __swift_bridge__$CursorIcon$is_text(ptr)
    }
}
extension CursorIcon: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_CursorIcon$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_CursorIcon$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: CursorIcon) {
        __swift_bridge__$Vec_CursorIcon$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_CursorIcon$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (CursorIcon(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CursorIconRef> {
        let pointer = __swift_bridge__$Vec_CursorIcon$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CursorIconRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<CursorIconRefMut> {
        let pointer = __swift_bridge__$Vec_CursorIcon$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return CursorIconRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<CursorIconRef> {
        UnsafePointer<CursorIconRef>(OpaquePointer(__swift_bridge__$Vec_CursorIcon$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_CursorIcon$len(vecPtr)
    }
}



