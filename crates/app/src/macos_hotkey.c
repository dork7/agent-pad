#include <Carbon/Carbon.h>

static bool matches(CFDictionaryRef hotkey, int key, int modifiers) {
    CFBooleanRef enabled = CFDictionaryGetValue(hotkey, kHISymbolicHotKeyEnabled);
    CFNumberRef hotkey_key = CFDictionaryGetValue(hotkey, kHISymbolicHotKeyCode);
    CFNumberRef hotkey_modifiers = CFDictionaryGetValue(hotkey, kHISymbolicHotKeyModifiers);
    int key_value, modifier_value;
    return enabled && CFBooleanGetValue(enabled)
        && hotkey_key && CFNumberGetValue(hotkey_key, kCFNumberIntType, &key_value)
        && hotkey_modifiers && CFNumberGetValue(hotkey_modifiers, kCFNumberIntType, &modifier_value)
        && key_value == key && modifier_value == modifiers;
}

OSStatus agent_pad_probe_hotkey(UInt32 key, UInt32 modifiers) {
    CFArrayRef system_hotkeys = NULL;
    OSStatus status = CopySymbolicHotKeys(&system_hotkeys);
    if (status != noErr) {
        return status;
    }
    for (CFIndex i = 0; i < CFArrayGetCount(system_hotkeys); ++i) {
        if (matches(CFArrayGetValueAtIndex(system_hotkeys, i), key, modifiers)) {
            CFRelease(system_hotkeys);
            return eventHotKeyExistsErr;
        }
    }
    CFRelease(system_hotkeys);

    // The plugin uses a nonexclusive registration, which succeeds even if an
    // exclusive owner will receive the shortcut. Probe first, with exclusivity.
    EventHotKeyID id = {0x536b5064, 1};
    EventHotKeyRef probe = NULL;
    status = RegisterEventHotKey(key, modifiers, id, GetApplicationEventTarget(),
                                 kEventHotKeyExclusive, &probe);
    return status == noErr ? UnregisterEventHotKey(probe) : status;
}
