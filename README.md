# twixel
This is julia's toy twitch bot, it's not really useful for anything except trying
out new rust things and learning stuff

`twixel_core` also happens to have the fastest twitch IRC message parser that
doesn't directly call SIMD intrinsics or use unsafe code <sub>(at least on my
Ryzen 5 2600)</sub>

I've thought about turning this into a general-purpose bot framework but that's too
much work and twitch is probably just gonna kill IRC soon soooooo...
