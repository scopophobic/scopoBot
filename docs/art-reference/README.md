# Artwork sources

Original user-supplied Glitch pose and reaction sheets are preserved here.
The runtime atlas is at `characters/glitch/atlas.png`.

Built-in image-generation edit prompt: Extract the existing Glitch Neutral Idle, Happy waving, Sleepy, and Happy jumping poses into a transparent 2×2 atlas; preserve hoodie, antenna, amber eyes and proportions; remove poster borders, labels, icons and ground shadows.

Mochi and Pip were generated with the built-in tool as transparent 2×2 atlases: a honey-and-cream puppy in a teal bandana, and a cheerful chestnut-haired human in coral and teal. Each has idle, happy, sleepy and seated master poses.

The compact Glitch revision uses dedicated transparent animation atlases in `characters/glitch/sprites/`; the earlier four-pose atlas is retained as source history and is no longer referenced by Glitch's manifest.

Motion atlas prompt: preserve the supplied Glitch identity; make a transparent four-column, four-row game sprite atlas with sequential breathing, blinking, walking, and waving frames; consistent full-body scale, fixed anchor, no poster text or scenery.

Reaction atlas prompt: same identity and grid; sequential sleeping, charging pulses, antenna/Wi-Fi searching, and happy hopping with confetti; no background, labels, borders, or ground shadows.

Additional state atlas prompt: same identity and grid; four sequential frames each of low battery with drooping antenna, busy typing with glasses and a small laptop, music with headphones and gentle bobbing, and curious/surprised expressions. Props are part of the character artwork, with no surrounding scene.

These are generated raster sprite assets based on the user's reference, not a skeletal rig. Their timing and frame selection are defined by the manifest and Director.

Battery-badge revision: edited the state atlas to remove all four large battery symbols in the first row, preserving antennae and the remaining twelve frames. The runtime draws a 14×7 badge only while charging or below 20%. Dancing uses the four headphone poses with faster, varied frame durations.
