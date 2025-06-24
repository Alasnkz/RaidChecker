## Raid Checker
**TL;DR: Checks WoW armory to see if players signed up to your raid are locked (had any kills on the selected bosses & difficulty), has any "gear problems" you may wish to check, not a recent enchant, doesn't have D.I.S.C. belt, doesn't have embelishments, doesn't have a socket on a ring. Or just basic stuff such as if they've killed the bosses at all, have the set item level requirement, or even has the raid buff!**

Source code: <https://github.com/Alasnkz/RaidChecker>
**DOWNLOAD: <https://github.com/Alasnkz/RaidChecker/releases/latest>**

Raid Checker was made to automate most of the pain points of NoP raid leading by checking out a player's character. You don't want any saved people to your raid? You can check to see if they've killed selected bosses on the difficulty you want to check! People aren't enchanted when you require them to be, you can check that! Players don't have the latest raid buff but it is possible for them to get it? You can see that!

Once you open the program, you will be greeted by some buttons (or an update notifier, because we all love latest software versions right?!)
## Settings
Settings is your main place to set the generic requirements to check against.

**You have "Item Requirements"**
    - You get the ability to set things such as the "head" slot required to be enchanted (or have a greater enchant).
    - You also have the ability to set the amount of sockets you require on each slot, i.e. 2 slots required on the neck.
    - Of course, with WoW now we also have "special items" as I like to call them, these can be the D.I.S.C. belt and other sorts. You can check to see if a slot is filled with one. 
        **VERY IMPORTANT: It will only check the current SEASON's special item. It will not check for the ring, and it won't check for D.I.S.C. in S3**.

**You have "Raid Requirements"**
    - This is your place to set kill requirements, i.e. I require 8/8 Heroic kills in my Mythic raid.
    - This is also where the average item level requirement is set.

**You have "check priority"**
    - This is related to the next setting also, but this is what should take precedence over other colours you may set in the character list.
    - People being saved is your main concern? Put it at the top and then you will be able to quickly glance at the list of characters and they'll be colour coded to the saved colour.

**You have "colour settings"**
    - As stated previously, this is where you set specific colours for things such as being saved to bosses, not matching the ilvl requirement, missing enchants, missing a special item, and so on.

## Check single character
You will get a text box, input the character (realm isn't needed but you can do it with Name-Realm, it will search the armory and allow you to click the character you want).
When the check is done you'll get a popup with the issues (if any).
You will also be asked if you want to check if they're saved to any bosses.

## Check sign-up URL
This checks against a **raid helper** event. It will check players that are not marked as absent, one by one and process them.
Should an issue occur like the person inputted the character's name wrong you will get the opportunity to type a name to replace theirs.
Of course, there's other things such as the person forgetting to input their realm with their sign up, in that case you will get a list of characters and you can click on the one that ideally matches theirs.

Once you have checked the raid, you will see a list of the characters on the left panel, colour coded to what you have set in the colour settings, if they're fine it'll be green.

## Download
Raid Checker features two different update mechanics:
    - Application update this will inform you that a Raid Checker update is avaiable **it will not download it for you**, if you click on download it'll bring you to the releases page.
    - Expansion data update this will inform you that a new version of the expansion data is avaiable, if you click download it **will download** the data and replace the current one.
    
It is **open source** please feel free to contribute and fix my bad code or something.

**The download and source code links are at the top.**

FAQ:
   Q: What is "expansion data"?
   A: Expansion data is all the enchant ids, raid ids, raid bosses, seasonal data such as special item ids, enchant ids and all sorts. Ideally, if there's an expansion data update you should probably get it.
