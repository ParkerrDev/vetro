import QtQuick

// Touchpad scrolling that carries on, for a list in a panel. Qt Quick moves a
// list exactly as far as the fingers move (after Omarchy's touchpad factor of
// 0.4, which assumes apps that glide by themselves) and stops it dead when
// they lift. Here a swipe moves it a little further the faster it is, and
// once the fingers lift the list glides on at the speed it had and slows to a
// stop, as GTK apps and browsers scroll on this desktop (2026-10-23: "the
// scrolling is also just the same speed everywhere").
//
// A mouse wheel, which has notches, is left to the list: Qt Quick already
// flicks it further the faster the wheel turns.
//
//   ListView { id: list; KineticWheel { view: list } }
WheelHandler {
  id: kinetic

  required property Flickable view

  // Omarchy scales touchpad scrolling to 0.4; this brings a slow swipe back to
  // about what the fingers moved, and a fast one up to twice that again.
  property real gain: 1.6
  property real boostAt: 2.5 // px per ms at which a swipe goes twice as far
  // Gliding: below this speed (px/s) at lift-off the list just stops.
  property real glideFrom: 120
  // The swipe's speed, px/s along contentY, smoothed over the last events.
  property real velocity: 0
  property real lastTime: 0

  target: null
  acceptedDevices: PointerDevice.TouchPad

  function clamped(y) {
    const top = view.originY;
    const bottom = Math.max(top, view.originY + view.contentHeight - view.height);
    return Math.min(bottom, Math.max(top, y));
  }

  onWheel: event => {
    const now = Date.now();
    // Touchpads send pixels; one that sends only notches is taken as 40 px a notch.
    const fingers = event.pixelDelta.y !== 0 ? event.pixelDelta.y : event.angleDelta.y / 120 * 40;
    if (fingers === 0)
      return;
    const elapsed = Math.min(100, Math.max(1, now - kinetic.lastTime));
    const fresh = now - kinetic.lastTime > 100;
    kinetic.lastTime = now;

    const speed = fresh ? 0 : Math.abs(fingers) / elapsed;
    const step = -fingers * kinetic.gain * (1 + Math.min(1, speed / kinetic.boostAt));
    kinetic.view.cancelFlick();
    kinetic.view.contentY = kinetic.clamped(kinetic.view.contentY + step);

    const instant = step / elapsed * 1000;
    kinetic.velocity = fresh ? instant : kinetic.velocity * 0.6 + instant * 0.4;
    kinetic.lift.restart();
    event.accepted = true;
  }

  // No event for a moment: the fingers have lifted, and the list glides on.
  // (A property: a pointer handler takes no children.)
  readonly property Timer lift: Timer {
    interval: 60
    onTriggered: {
      if (Math.abs(kinetic.velocity) >= kinetic.glideFrom)
        kinetic.view.flick(0, -kinetic.velocity);
      kinetic.velocity = 0;
    }
  }
}
