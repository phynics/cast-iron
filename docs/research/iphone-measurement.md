# iPhone measurement session

For [#423](https://github.com/phynics/cast-iron/issues/423). One session on a real iPhone. It gives the numbers that decide the format 5 work ([#264](https://github.com/phynics/cast-iron/issues/264), [#183](https://github.com/phynics/cast-iron/issues/183)) and the search work, and it checks the VoiceOver question of [#419](https://github.com/phynics/cast-iron/issues/419). It also covers the physical-device part of [#12](https://github.com/phynics/cast-iron/issues/12).

**You need:** your iPhone, Wi-Fi, and a charger. You do not need a Mac or Web Inspector for any step. The app records its own numbers, and the Copy button gives them to you as text.

**Time:** about 30 minutes of your attention. The two library downloads (about 207 MB each) add waiting time that depends on your Wi-Fi. If you are short on time, skip Part 1. You then lose only the Safari-tab install time and the storage-eviction check, and you can do Part 1 on another day.

Nothing is sent anywhere. The numbers stay on the phone until you copy and paste them.

## Before you start (3 minutes)

1. **Check that the build has Diagnostics.** Open the app, then Settings. Under "This app", a row named "Diagnostics" must be there. If it is not, you have an older build. Stop and ask for a newer one.
   - The published site (`https://phynics.github.io/cast-iron/`) has Diagnostics only after a release promotion ([docs/releasing.md](../releasing.md)). Until then, the build must come from an HTTPS address. A plain `http://` address on your network does not work: iOS gives the service worker and the checksum code only to HTTPS pages.
   - The safest way to test before a promotion is the reviewed `dist/` on a separate temporary HTTPS address, at the path `/cast-iron/`, with no added scripts or analytics. A separate address has its own service worker, caches and storage, so it cannot touch the production site. Leave the production site and the `release` branch alone.
   - Write down the app line and the library id from Settings > Diagnostics ("Device and library"). Label the results as a **fresh preview install**. They are not an upgrade check of the production site.
2. **Free the phone.** Settings > Battery: Low Power Mode off. Join Wi-Fi. Close other apps.
3. **Keep the screen on.** Settings > Display & Brightness > Auto-Lock > Never. iOS pauses a page when the screen locks, and a download that pauses gives a wrong install time. Set it back at the end.
4. **Write down the phone.** Settings > General > About: Model Name, and iOS Version. The app cannot read the iOS version reliably.
5. **Open Notes** and start a note called "Cast Iron results". You paste into it after some steps.

## Part 1: Install in a Safari tab (about 8 minutes, can be skipped)

This copy is the one that has no Home Screen icon. It gives the install time in a tab, and it is the copy for the eviction check in Part 7.

1. Open Safari. Go to the app address. Do not add it to the Home Screen yet.
2. Tap **Download**. Keep Safari open and on screen until the progress bar ends and Home appears. Do not switch apps.
3. Note how long it took, by the clock on your phone, as a rough check. The app measures it exactly.
4. Tap **Settings** (bottom or top bar), then **Diagnostics** (under "This app", "Timings and storage").
5. Tap **Copy results**. Open Notes and paste under the title "S, Safari tab".
6. Leave this Safari tab alone from now on. Do not open the app address in Safari again until Part 7. Opening it resets the clock that Part 7 checks.

## Part 2: Install on the Home Screen (about 8 minutes)

1. In Safari, tap the Share button, then **Add to Home Screen**. Check that **Open as Web App** is on. Tap **Add**.
2. Close Safari. Tap the new **Cast Iron** icon on your Home Screen. This is a second app with its own storage, so it shows the install screen again. That is expected.
3. Tap **Download**. Keep the app on screen until Home appears.
4. Tap **Settings**, then **Diagnostics**. Under "Install", read "total". Under "Storage and memory", read "persistent storage".

You now have two installs: the tab (Part 1) and the Home Screen app (this part). The Diagnostics text of each says "shown as browser tab" or "shown as Home Screen app", so the two are easy to tell apart.

## Part 3: Cold starts (3 minutes)

A cold start means the app was fully closed first.

Do this three times:

1. Swipe up from the bottom edge of the screen and pause. The app switcher opens.
2. Swipe the Cast Iron card up and off the top of the screen.
3. Wait 5 seconds.
4. Tap the Cast Iron icon.
5. Count the seconds from your tap until you can tap a category on Home. Write the number down. A rough count is fine.
6. Do nothing else in the app. No search.

Each start is recorded as its own "launch". Diagnostics lists them all under "Earlier launches".

The app's own number starts when the page starts. It does not include the time iOS needs to open the app before that. Your count does. Both numbers are useful, so write yours down.

*If you have time left at the end:* restart the iPhone, open the app once, and write down the seconds. That is the slowest start there is.

## Part 4: Search and a large page (6 minutes)

This is one launch. Do all of it without closing the app. The text you copy at the end has the details for this launch only.

The search icon at the top opens **Quick search**, a small box with suggestions. The app measures its suggestions and, separately, the **full results page** that you reach from it. You use both here.

1. Close the app fully, as in Part 3. Tap the icon. Wait for Home.
2. Open **Settings > Diagnostics**. Write down the launch number under "This launch" (for example `#4`). Opening this screen also switches on the measuring of pages for this launch. Go back to Home.
3. **First search.** Tap the search icon at the top. Type `grapple`. The suggestions you see are Quick search. Press the **Search** key on the keyboard. This opens the full results page. If you see the words "Loading search index…", write that down.
4. **Check that the numbers are being recorded.** Open Settings > Diagnostics. Under "Full results searches in this launch" it must show at least 1 search, and under "Quick search suggestions" it must show at least 1. If either says "none yet", stop and write that down. Then go back to the search.
5. **Typical search.** Use the search field on the results page itself (not the icon at the top). Clear it and search for each of these, waiting for the results each time: `fireball`, `cure light wounds`, `initiative`, `dragon`. Then search for `a`, and `wizard`. These are full-results measurements. Quick search has already done part of the work for the first query, so the first number is a little better than a first search from nothing.
6. **Large page.** Open the Wizard class page. Go to Home > Classes > Wizard, or search for `wizard` and tap the result that is marked Class. Wait until the text shows. Write down how long it took, roughly.
7. Scroll to the very bottom with repeated flicks. Scroll back to the top. Tap two links on the page and come back with Back.
8. Watch for these and write down each one you see:
   - the page goes white or blank for a moment;
   - the page jumps back to the top by itself;
   - the app returns to Home, or to the install screen, without you doing it;
   - scrolling stutters or stops answering;
   - Safari or iOS shows a message about the page using too much memory.
9. Open **Settings > Diagnostics** again. Check the launch number under "This launch".
   - **The same number as in step 2:** the page was not loaded again. Good.
   - **A higher number:** the app was loaded again while you worked. Treat this as a **suspected restart**. A new number proves only that the page started again. It does not prove why. iOS closing the app because of memory is one possible cause. Write down what you did just before it, and which step you were in.
10. Check that "Pages opened" lists Wizard. Tap **Copy results**. Paste into Notes under the title "A, main launch".

**What the numbers mean.** The page has no way to ask iOS how much memory it uses. Safari does not give web pages a memory number. So this part measures what the app can see:

- **elements** and **characters** show how big the page is.
- **WebAssembly memory allocated (high-water mark)** shows the most memory the search and library code asked for. It never shrinks. It is not live data, and it is not the memory of Safari or of the whole page.
- **whether the launch number went up** (step 9) is a hint about memory, not a proof.

**What a search time means.** "Painted after" counts from the start of the run until two frames after the results were shown, so it includes drawing them. "Work" stops just before the results are handed to the screen. The typing pause before the run (80 ms for results, 100 ms for suggestions) is not in either number. "Median" is the middle value (the lower one when there is an even count). With fewer than 20 searches after the first, the 95th percentile is just the slowest one, so read the sample count too.

## Part 5: VoiceOver (6 minutes)

This is the check from [#419](https://github.com/phynics/cast-iron/issues/419). When a page opens, the app first shows the page title and the words "Loading entry…". When the text arrives, the title is drawn again. The question is what VoiceOver says at each point, and whether it says anything when loading ends.

**Set up VoiceOver.**

1. Settings > Accessibility > VoiceOver. Turn it on.
2. In the same screen, open **Visual** and turn on **Caption Panel**. A bar then shows on screen what VoiceOver says, so you can read it back and copy it.
3. The gestures you need:
   - Swipe right with one finger: next item. Swipe left: previous item.
   - Double-tap anywhere: open the selected item.
   - Scroll with three fingers.
   - To turn VoiceOver off later: Settings > Accessibility > VoiceOver.

**Do the check.** Do it right after a cold start. The first page you open is the slowest, so its loading words last longest.

1. Close the app fully. Tap the icon. Wait for Home. Open **Settings > Diagnostics** once, then go back to Home. This switches on the measuring of pages for this launch, which you need for step 6.
2. Swipe right until VoiceOver reads **Classes**. Double-tap. Swipe right until it reads **Wizard**. Double-tap.
3. Listen, and write down the exact words you hear (the Caption Panel shows them):
   - **a.** The first thing VoiceOver says after you double-tap.
   - **b.** Whether it says "Loading entry".
   - **c.** Whether it says anything when the text arrives. Silence is a finding, so write "nothing".
   - **d.** Swipe right once. What does it read? The page title (Wizard, heading) is the good answer. The top bar or the breadcrumb is not.
   - **e.** Swipe right five more times. Does it read the class text, one item after the other, without jumping back to the top?
4. Go Back. Open **Wizard** a second time. This time the page opens faster. Write down whether you still hear the loading words, and what comes first.
5. Open a small page too. From Home, open Feats and any feat. Write down a, b and c again.
6. Turn VoiceOver off. Open **Settings > Diagnostics**. Under "Pages opened", read how long each page took to show. A page that showed in under 100 ms gave VoiceOver almost no time to say the loading words. Tap **Copy results**. Paste into Notes under "V, VoiceOver".

VoiceOver slows the app down. Do not use the speed numbers of this launch.

## Part 6: Offline restart (3 minutes)

1. Open Control Center (swipe down from the top right corner). Turn on Airplane Mode. Check that the Wi-Fi symbol is off too. If Wi-Fi came back on, turn it off.
2. Close the app fully, as in Part 3.
3. Tap the Cast Iron icon.
4. Write down what you see. Home with the word "Offline" in the top bar is the correct result. The install screen, a blank page, or an error is a failure. Write down what it said.
5. Search for `fireball`. Open Fireball. Open Settings and tap **Verify library**. Write down the result.
6. Open **Settings > Diagnostics**. Check that it says "online at start: no". Tap **Copy results**. Paste into Notes under "O, offline".
7. Turn Airplane Mode off.

## Part 7: Does Safari remove the stored library? (a later check, 1 minute)

Safari can remove the stored data of a site. This check records what happens to the Safari-tab copy on your phone. It does not tell you why, and you must not claim a cause from it.

What WebKit documents (check these pages if you want the detail):

- **Seven days of Safari use, not seven calendar days.** Safari's tracking prevention deletes a site's script-written storage after seven days of *Safari use* in which you did not interact with the site. Days when you do not use Safari at all do not count. See [Full Third-Party Cookie Blocking and More](https://webkit.org/blog/10218/full-third-party-cookie-blocking-and-more/) and [Tracking Prevention in WebKit](https://webkit.org/tracking-prevention/).
- **The Home Screen exemption covers only that rule.** A Home Screen web app is exempt from the seven-day removal. It is not protected from every kind of removal.
- **Low storage can remove data too.** Safari removes the data of sites on its own when the device runs short of space or a quota is passed, oldest use first. Data that a site marked as persistent is exempt. See [Updates to Storage Policy](https://webkit.org/blog/14403/updates-to-storage-policy/). A cleared copy therefore does not prove that tracking prevention did it.

What to do:

- Leave the Safari-tab copy from Part 1 alone. Do not open it in Safari until the check.
- Use Safari as you normally do for a week or more. Do not tap or type on the Cast Iron site in that time. Use the Home Screen app as you like.
- At the check, write down, **before** you open the Safari-tab copy:
  - the date of Part 1 and today's date;
  - on about how many of those days you used Safari;
  - whether you opened or touched the Cast Iron site in Safari in between;
  - roughly how full the phone is (Settings > General > iPhone Storage).
- Now open the Safari-tab copy and write down what you see:
  - **The library opens on Home:** the data was still there.
  - **The install screen shows ("Install the rules library"):** the data was gone.
- Do the same with the Home Screen app, as a second observation.
- Compare the "persistent storage" and "storage used ... of ..." lines of S (tab) and A (Home Screen app). Write down if they differ.

Report what you saw and the facts above. A copy that survived after a week means little if you used Safari on only two of those days. A copy that disappeared does not tell you whether tracking prevention or low storage did it.

After the check you can remove the tab copy: Settings > Apps > Safari > Advanced > Website Data, search for the site, and delete it.

## Send the results (2 minutes)

Paste the Notes into the chat with Claude, or into a comment on issue #423. Add these answers from your own notes:

```
Phone model:
iOS version:
Wi-Fi or cellular:
Part 1 tab install felt like (minutes):
Part 2 Home Screen install felt like (minutes):
Part 3 cold start counts (seconds): 1) 2) 3)
Part 3 after a restart of the phone (seconds, optional):
Part 4 "Loading search index…" seen on the first search? yes / no
Part 4 Wizard page opened after about (seconds):
Part 4 problems seen (blank, jump, restart, stutter, memory message):
Part 4 launch number before / after (suspected restart? what you did before it):
Part 5 VoiceOver a, b, c, d, e (exact words):
Part 5 second open of Wizard:
Part 5 small feat page a, b, c:
Part 6 offline result (Home / install screen / error text):
Part 6 Verify library result:
Part 7 (later): date of Part 1, date of the check, days you used Safari, touched the site in Safari? (yes/no), how full the phone is, tab copy kept / cleared, Home Screen app kept / cleared:
```

The pasted report has no names, no search words and no account data. It holds the browser's user-agent line, which names the phone type and iOS family, and counts and times. Quick search keeps your recent searches in a different place, and they are not in the report.

If you want the record gone from the phone, open Settings > Diagnostics and tap **Clear diagnostics**. That removes this record and nothing else: not the library, not your stars. Use one Cast Iron window at a time during the session. The Safari tab and the Home Screen app keep separate records, so they do not mix.

Then set Auto-Lock back (Settings > Display & Brightness).

---

# For the person who reads the results

## What each measurement records, and how

| Measurement | Where it shows | How it is recorded |
|---|---|---|
| Install duration | Diagnostics, "Install": total, and scan, download, verify, activate; the download rate counts only the bytes fetched | `install::install` reads `performance.now()` at each phase boundary and stores one record (`diagnostics::InstallRecord`) after activation. A resumed install shows how many files were already there. |
| Persistent storage | "Storage and memory", and "persistent storage asked for" under Install | `navigator.storage.persisted()` now. The install records the answer of `navigator.storage.persist()`. |
| Storage used and quota | "Storage and memory" | `navigator.storage.estimate()` when the page is opened or refreshed. |
| Cold start to usable Home | "This launch": app code running, library files checked, catalog read, library ready, Home on screen | Marks in `app.rs`, `library.rs` and `home.rs`, each as `performance.now()` since the page began. Home is marked two animation frames after it mounts, and only when the launch opened on Home. |
| Search index load | "This launch": search index read, decode, ready at | `Library::search_index` times the cache read and the decode of `search.bin`. Home loads it in the background; "ready at" is when it was done. |
| First and typical search | "Full results searches" and "Quick search suggestions" | Each screen records every run: query length, result count, wait for the index, engine time, work until the results are handed to the screen, and total until two animation frames after that (painted). A run that a newer one replaced is dropped. The first search is kept apart. The typing pause (80 ms results, 100 ms suggestions) comes before these numbers. The query text is not kept. |
| Large page | "Pages opened in this launch" | The Entry screen records the time from opening to two animation frames after the text arrives, then the element count and the text length of the whole document. Only after Diagnostics has been opened in the launch, so an ordinary reader never pays for the count. |
| Memory | "Storage and memory" | The size of the WebAssembly linear memory (`wasm_bindgen::memory()`): memory allocated so far, a high-water mark that never shrinks, not live objects and not the process total. `performance.memory` on browsers that offer it (Chromium); Safari does not, and the text says so. |
| Offline restart | "This launch": online at start, Home on screen | `navigator.onLine` at the start of the page, and whether Home was reached. |

The last 8 launches are stored on the device (IndexedDB, `meta` key `diagnostics`). Only the current launch has the search and page detail. A write reads, changes and writes the record in one IndexedDB transaction, so two windows cannot take the same launch number. A launch number that rises is a new page load, nothing more.

## What needs a Mac

Nothing in the checklist does. Two things cannot be measured on the phone alone:

- **Memory of the page itself** (DOM, images, Safari's own process). Safari offers no number to a web page. Part 4 uses a check for a suspected restart instead. A Mac with Web Inspector (Develop menu, the iPhone, Timelines, Memory) could take the real figure. It is worth doing only if Part 4 shows blank pages or restarts.
- **Time before the page starts**, which includes iOS opening the web view. Part 3 uses a stopwatch count for it.

## Where the results go

[#423](https://github.com/phynics/cast-iron/issues/423) asks for the results in `docs/status.md`, with device, iOS version and build, and for follow-up issues filed from the numbers. The session that receives the paste does both. Part 5 findings go into `docs/decisions.md`, as [#419](https://github.com/phynics/cast-iron/issues/419) asks.
