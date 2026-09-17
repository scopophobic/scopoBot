# Native acceptance checklist

Run this on each release platform; compilation is not a substitute for desktop behavior testing.

- Launch the packaged app from another directory. Confirm assets resolve.
- Confirm only the character is visible on the desktop; no opaque rectangle.
- Put a normal application behind the character; confirm always-on-top behavior.
- Drag the character, quit through the tray, relaunch, confirm position and scale restore.
- Turn click-through on; interact with the underlying app; restore interaction through the tray.
- Open and close Character Lab repeatedly; the companion should survive.
- Pause, step a frame, resume. Change integer scale and backdrop.
- Simulate offline and low battery together; network reaction plays first, then battery slump.
- Simulate CPU >80% for at least five seconds, then <60% for ten seconds.
- Rapidly toggle network and charging; observe cooldowns and bounded queue.
- Edit a pack; valid changes reload, invalid JSON or image dimensions preserve the prior pack.
- Turn local readings off; stale readings should clear. Weather is controlled separately.
- With weather off, verify the process makes no application network requests.
- Test optional city lookup, selection, network failure, and disabling weather.
- Test missing battery and unsupported headphone/Bluetooth hardware.
- Disconnect a display and use Reset position if necessary.
- Measure idle CPU and resident memory with Character Lab closed, then under Saver/Ultra.
- Launch terminal mode; test keyboard controls, resize, quit, and Ctrl+C restoration.
- On Windows, test both `scopobot.exe` and `scopobot-terminal.exe` from the portable ZIP.
