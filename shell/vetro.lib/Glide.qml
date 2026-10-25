import QtQuick

// A value glided to where it is sent. Sent somewhere new on the way, it keeps
// the speed it has and curves round to the new place, where a timed animation
// would start again from rest and jerk: for what changes often, as the volume
// does while its key is held. Its length is one of the look's, Look.moderate
// unless told, and never quite 0, which this animation cannot take.
SmoothedAnimation {
  property int length: Look.moderate

  velocity: -1
  duration: Math.max(1, length)
}
