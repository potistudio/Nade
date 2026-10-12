#import <AppKit/AppKit.h>

static id scrollMonitor;
static BOOL directGestureActive = NO;
static BOOL momentumInput = NO;
static NSRect timelineBounds;
static NSPoint lastScrollPoint;

BOOL nade_scroll_gesture_active(void) {
	return directGestureActive && NSPointInRect(lastScrollPoint, timelineBounds);
}

BOOL nade_scroll_input_is_momentum(void) {
	return momentumInput && NSPointInRect(lastScrollPoint, timelineBounds);
}

void nade_set_timeline_scroll_bounds(double x, double y, double width, double height) {
	timelineBounds = NSMakeRect(x, y, width, height);
}

void nade_install_scroll_observer(void) {
	if (scrollMonitor != nil) {
		return;
	}
	scrollMonitor = [NSEvent addLocalMonitorForEventsMatchingMask:NSEventMaskScrollWheel
		handler:^NSEvent *(NSEvent *event) {
			NSView *view = event.window.contentView;
			if (view == nil) {
				return event;
			}
			NSPoint point = [view convertPoint:event.locationInWindow fromView:nil];
			point.x -= NSMinX(view.bounds);
			point.y = view.isFlipped ? point.y - NSMinY(view.bounds) : NSMaxY(view.bounds) - point.y;
			lastScrollPoint = point;
			momentumInput = event.momentumPhase != NSEventPhaseNone;
			if (momentumInput) {
				directGestureActive = NO;
			} else if (event.phase & (NSEventPhaseMayBegin | NSEventPhaseBegan | NSEventPhaseChanged | NSEventPhaseStationary)) {
				directGestureActive = YES;
			} else if (event.phase & (NSEventPhaseEnded | NSEventPhaseCancelled)) {
				directGestureActive = NO;
			}
			// Observation only: never consume an application-wide event.
			return event;
		}];
}
