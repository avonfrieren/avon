# Speedrun Tool Additions (srta) [OLD TODO]
Fork or new Speedrun Tool mod in which you can :

## Practice sheet implementation
* Colored final time cp / il (doable)
* Condensed time map / image in game (doable)
* Complete implementation / synchronisation of google 
sheet in game (hardest ?)

## Others
* Cancel a PB (doable)
* Set a PB manually (doable)
* Set more than 9 states (hard ?)
* Save states permanently (hardest ?)

## Hide timer
Make possible to hide the timer with parameters such as :

* Show only when state run is finished (doable)
* Toggle button timer (doable)

## Show delta
For each room show the delta even if it's + 
something, not only the timer being gold but
also the actual delta -X or +X

* Toggle button delta's (doable)

## GPS
Trackmania 2020 GPS style mod in which you can :

## Real time physics simulation
* Setup a shadow Madeline to do a route (hard ?)
* Make Madeline disapear / invincible without killing
*  the golden type berry's (doable ?)
* Make the camera lock on the shadow Madeline (doable ?)
* Loop in that state unless you press a button (doable)

## Video
* Play / implement a video in Celeste (doable ?)

# Theo score points (tsp) [DONE WITH CHANGES]
Theo gameplay is cool, but most of the time you can throw him
far away and get him later, or entirely left him behind in
some rooms, it should be more fun to play and to watch to "force"
in some way taking risk and findings strats to do difficult or
unusual things. So the principle is too encourage that behavior
by making a scoring system that push people in that way.

## Scoring ideas
How close Theo was from Madeline since the beginning of the room
add this too a counter, problem : if you afk while holding Theo 
you farm points, need further thinking

## Implementation
It should be possible to calculate it from Madeline and Theo's
position at any time

# Deathless room randomizer (drr) [TODO]
The mod have to be simple, efficient, and serve one and only purpose
Every bonus features like other stats that are not needed for the core of the
mod should be done later or even not done at all.
## Why ?
Train deathless gameplay by putting your brain on the side and only
play the game endlessly.
## What it does ?
While playing on repeat the mod itself make statistics about what room
you struggle to make you play them more, make statistics about your gameplay
and of course randomize the rooms each time you take a transition ? Or
in another timing it depend on how it will be implemented if possible.
## How ?
* Automatically tp you in another room on transition ?
* Create a map with every room, the map is recreated when you have finished this run
with the statistics stored in mind tu put more of the rooms you struggled with
## On what ?
A single map you've chosen for simplicity, since the goal of this mod is to train
yourself on a specific map for deathless purpose.

# Real time input recorder (rtir) [TODO]
Simple mod to save you from not forgeting to record or stream your grind
and lose/need to redo it for a video, should not be applicable for hist/gbnet ofc
because it's saving the input's in order to replay it, it's not a proof you didn't
just TAS it yourself. 
## Why ?
I had this
idea while seeing for a 1k time now the Motion Smoothing mod and think : "What if
I can do my grind normally without accomodating myself to a higher frame rate because
I can't, the rest of my play can't have that like vanilla speedrun's, and just replay
it later like a TAS file with Motion Smoothing ?".
## How ?
* Write and save input on a TAS file in real time
* Record a video ingame by playing the TAS file directly ? Would require CelesteTAS,
maybe not necessary to add a dependency when you can just record your screen normally
