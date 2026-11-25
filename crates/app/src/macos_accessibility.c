#include <ApplicationServices/ApplicationServices.h>
#include <stdbool.h>

bool skills_pad_accessibility_granted(void) {
    return AXIsProcessTrusted();
}

void skills_pad_prompt_for_accessibility(void) {
    const void *key = kAXTrustedCheckOptionPrompt;
    const void *value = kCFBooleanTrue;
    CFDictionaryRef options = CFDictionaryCreate(kCFAllocatorDefault, &key, &value, 1,
                                                  &kCFTypeDictionaryKeyCallBacks,
                                                  &kCFTypeDictionaryValueCallBacks);
    if (options) {
        AXIsProcessTrustedWithOptions(options);
        CFRelease(options);
    }
}
