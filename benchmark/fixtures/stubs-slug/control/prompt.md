Some web addresses are too long for the magazine's newsletter.

Add `short_slug(title, limit)` to `src/lib.rs`. It returns the slug of the
title, cut to at most `limit` characters. It cuts only where a `-` stands, so
no word is split, and the result never ends with a `-`. When the first word
alone is longer than `limit`, that word is cut at `limit` characters.

Cover the new behaviour with tests beside the ones already there.
