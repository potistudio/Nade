#import <AppKit/AppKit.h>

typedef void (*NadePinchCallback)(double delta);

static id g_event_monitor = NULL;
static NadePinchCallback g_pinch_callback = NULL;

void nade_setup_pinch_monitor(NadePinchCallback callback) {
    if (g_event_monitor != NULL) {
        return;
    }
    g_pinch_callback = callback;
    g_event_monitor = [NSEvent addLocalMonitorForEventsMatchingMask:NSEventMaskMagnify
                                                           handler:^NSEvent *(NSEvent *event) {
        if (g_pinch_callback != NULL) {
            g_pinch_callback([event magnification]);
        }
        return event;
    }];
}

void nade_teardown_pinch_monitor(void) {
    if (g_event_monitor != NULL) {
        [NSEvent removeMonitor:g_event_monitor];
        g_event_monitor = NULL;
        g_pinch_callback = NULL;
    }
}
