# Vendored upstream sources

These are pristine source snapshots. Make project-specific changes as patches outside
this directory; do not edit a vendored tree in place.

| Component | Upstream ref | Commit | GitHub archive SHA-256 |
| --- | --- | --- | --- |
| Shairport Sync | 5.5.2 | `7bad231c18368dbd26f298577f6210e36e4b0797` | `eab1fa095e34676d05f68e38d86501d5afa3fc46f83f044859d4d125d526daec` |
| NQPTP | 1.2.8 | `c925f27c1fd12e4033ac477e5a405969b0b0260b` | `d2c2fe5d2574d447a817b1585e82c38f4c98774dac8284e5a3f17e188a3a75f9` |

Archives were fetched from `https://github.com/mikebrady/<component>/archive/<commit>.tar.gz`
and extracted with `tar --strip-components=1`.

The sibling `../openairplay2-echo` project is a behavioural/package reference only;
no source was copied from it. Preserve each upstream component's own licence files.
NQPTP is GPL-licensed, so distribution of a release ZIP needs a separate licence and
source-compliance review.
