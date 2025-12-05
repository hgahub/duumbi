# PRD - Cél és Hatókör (Duumbi MVP)

## Cél és Hatókör

A Duumbi egy Mesterséges Intelligencia (MI) által támogatott ingatlanhirdetési és -kereső platform. Ennek a dokumentumnak a célja, hogy meghatározza a **Duumbi platform Minimum Viable Product (MVP) verziójának** termékkövetelményeit.

**Fő termékcél (MVP):**

Egyszerű, gyors és hatékony digitális megoldást nyújtani magánszemélyek és kis ingatlanirodák számára Magyarországon ingatlanok hirdetésére és keresésére. Az MVP egy **webalkalmazás (React SPA)** lesz, amely MI-alapú segítséget kínál a hirdetések létrehozása során és intelligens szűrési lehetőségeket a keresőknek. Célunk egy felhasználóbarát portál létrehozása, kezdetben a magyar piacra fókuszálva, amely a hirdetők és keresők legsürgetőbb problémáira ad választ. A platform a `duumbi.io` domainen lesz elérhető.

**Miért épül?**

Jelenleg nincs a piacon olyan integrált, kifejezetten Magyarországra lokalizált és MI-vel támogatott megoldás, amely végponttól végpontig segítené a magánhirdetőket minőségi hirdetések létrehozásában, és segítené a keresőket a releváns ajánlatok megtalálásában. A Duumbi célja ennek a piaci résnek a betöltése.

**Hatókör (MVP Hatóköre):**

Az MVP a következő fő területekre összpontosít:
- Felhasználói portál alapvető információs tartalommal és ingyenes keresési lehetőséggel.
- Regisztráció és felhasználói fiókkezelés a prémium funkciók eléréséhez.
- MI-alapú hirdetéslétrehozó asszisztens magánhirdetők számára (szövegírás, alapvető képelemzés, egyszerűsített árajánlás).
- Intelligens keresési funkciók életmód alapú szűréssel.
- Fizetős csomag a hirdetők számára.
- Fizetős csomag az ingatlan keresők számára.
  
---

# PRD - Célközönség és Felhasználói Perszónák


A Duumbi MVP elsődlegesen a következő csoportokat célozza a magyar piacon (a Duumbi Üzleti Terv alapján):

1. **Anna, az Első Eladó (Magánhirdető - B2C):**
	- **Kor:** 28-35 év.
	- **Foglalkozás:** Fiatal szakember, digitálisan aktív.

	- **Cél:** Első ingatlanának gyors és jó áron történő eladása vagy bérbeadása, a közvetítői költségek minimalizálásával.

	- **Fájdalompontok:** Nincs tapasztalata hirdetésírásban, bizonytalan a jó fotókban, bizonytalan az árazásban, nincs ideje szakértő segítségét kérni.

	- **Technológiai affinitás:** Magas, nyitott az új online eszközökre.

	- **Duumbi értékajánlata számára:** "Professzionális hirdetés egy kávé áráért" – MI asszisztens, amely segít minőségi hirdetést létrehozni.

2. **Péter, a Kis Ingatlaniroda Tulajdonos (B2B-KKV):**
	- **Kor:** 35-50 év.
	- **Cégméret:** 1-5 fős iroda.

	- **Cél:** Hatékonyabban kezelni a hirdetéseket, több ügyfelet elérni, időt megtakarítani az adminisztrációval.

	- **Fájdalompontok:** Időhiány, manuális és időigényes hirdetésfeltöltés több portálra, korlátozott marketing költségvetés.

	- **Technológiai affinitás:** Közepestől magasig, szoftverekre fogékony, hatékonyságnövelő eszközöket keres.

	- **Duumbi értékajánlata számára:** "White-label MI eszköz, ami órákat spórol" – MVP-ben a magánhirdetői funkciókat hatékonyabban tudja használni.

3. **Balázs, a "Bérlőből Tulajdonos" Kereső (Ingatlankereső - B2C):**
	- **Kor:** 24-40 év.
	- **Cél:** Megtalálni az első saját lakását vagy az ideális bérleményt, amely megfelel az igényeinek és életstílusának.

	- **Fájdalompontok:** Túl sok irreleváns hirdetés a nagy portálokon, nehézkes szűrés, nincs kontextus az árakhoz, időigényes keresés.

	- **Technológiai affinitás:** Magas, mobil-fókuszú, személyre szabott és gyors megoldásokat vár el.

	- **Duumbi értékajánlata számára:** "Kevesebb keresés, pontosabb találatok" – Életmód alapú szűrő, személyre szabottabb ajánlatok (MVP-ben egyszerűsítve).

---

# PRD - Probléma és Lehetőség

## Piaci Probléma (A Duumbi Üzleti Terv alapján)

**Magánhirdetők számára:**

- Gyakran hiányoznak a professzionális eszközeik és tudásuk a hatékony ingatlanhirdetések létrehozásához (pl. rossz minőségű fotók, pontatlan vagy hiányos leírások, nem optimális árazás).

- A hirdetés feladásának folyamata több portálra időigényes és bonyolult lehet.

- Nehéznek találják, hogy hirdetéseik kitűnjenek a tömegből.


**Ingatlankeresők számára:**

- A nagy ingatlanportálok gyakran túl sok irreleváns találatot jelenítenek meg; a szűrési lehetőségek korlátozottak vagy nem elég intelligensek.

- A megfelelő ingatlan megtalálása időigényes és sok manuális böngészést igényel.

- Nehéz összehasonlítani az ajánlatokat és piaci kontextusba helyezni az árakat.

## Lehetőség

  A Duumbi célja értékteremtés egy MI-alapú platform fejlesztésével, amely:

- **Hirdetőknek:** Intelligens segítséget nyújt a teljes hirdetésfeladási folyamat során, a minőségi tartalom (szöveg, képjavaslatok) létrehozásától az árajánlásokig, ezzel növelve hirdetéseik hatékonyságát és csökkentve az eladáshoz/bérbeadáshoz szükséges időt.

- **Keresőknek:** Személyre szabottabb és relevánsabb találatokat biztosít intelligens szűrők és ajánlások révén, megkönnyítve és felgyorsítva a keresési folyamatot.

- **A Piacnak:** Egy innovatív, technológia-vezérelt megoldást kínál, amely betölti a jelenlegi piaci rést a magyar (és később az európai) ingatlanpiacon.

---

# PRD - Funkciók MVP

Az MVP a következő fő funkciókra összpontosít, MoSCoW szerint priorizálva:

## Must-have (Elengedhetetlen az MVP-hez):

- **M1: Alapvető Felhasználói Portál:**

	- Nyilvános kezdőoldal alapvető információkkal a Duumbi szolgáltatásairól.

	- Ingyenes, korlátozott ingatlankeresési lehetőség (pl. kevesebb szűrő, korlátozott találati lista).

	- Regisztrációs és bejelentkezési felület.

- **M2: Alapvető Felhasználói Fiók:**

	- Alapvető profiladatok megtekintése és szerkesztése.

	- Aktuális hirdetések listázása (hirdetők számára).

	- Előfizetési csomag állapotának megjelenítése.

- **M3: MI Hirdetés Asszisztens (Alap):**

	- Strukturált kérdőív az ingatlan részleteiről.

	- MI-alapú hirdetésszöveg-generálás a megadott adatok alapján.

	- Alapvető képelemzési visszajelzés (pl. túl sötét, homályos – egyszerűsítve).

	- Egyszerűsített árajánlás (pl. közeli, hasonló hirdetések átlagárai alapján, ha az adatforrás megvalósítható az MVP-hez, egyébként manuális bevitel).

	- Hirdetés előnézete és közzététele a Duumbi portálon.

- **M4: Intelligens Keresés (Alap):**

	- Szűrés alapvető ingatlanparaméterekre (ár, méret, szobák, típus, elhelyezkedés).

	- Életmód alapú szűrési lehetőség (pl. "első otthon," "családnak," "befektetés" – 2-3 előre definiált opció az MVP-ben).

	- Keresési találati lista és ingatlan adatlapok megjelenítése.

- **M5: Fizetési Csomagok

	- Egy csomag meghatározása hirdetőknek (pl. több képfeltöltés, kiemelt hirdetés a Duumbin, hosszabb megjelenési idő) az üzleti terv alapján.
	
	- Egy csomag az ingatlan keresőknek. Több AI használat, Mély kutatás.

	- Integráció egy fizetési átjáróval (pl. Stripe) az előfizetés kezeléséhez. (_Ha túl bonyolult az MVP-hez, az MVP ingyenesen is indulhat, későbbi monetizációval._)

## Should-have (Fontos, de erőforráshiány esetén elhalasztható):

- **S1: Egyszerűsített Ajánlórendszer Keresőknek:**

	- Nagyon alapvető "ezek is érdekelhetnek" ajánlások a keresési eredményekben vagy az adatlapokon, a keresési előzmények és preferenciák alapján.

- **S2: Mentett Keresések és Értesítések:**

	- Keresési kritériumok mentése.

	- Alapvető e-mail értesítés a mentett kereséseknek megfelelő új hirdetésekről.

- **S3: Alapvető Hirdetési Statisztikák:**

	- Alapvető statisztikák a hirdetőknek saját hirdetéseikről (pl. megtekintések száma a Duumbin).

## Could-have (Jó, ha van, ha az idő/erőforrás engedi, de elhagyható):

- **C1: Alaprajz Feltöltési Lehetőség:** Kézzel rajzolt vagy egyszerűsített digitális alaprajz feltöltése.

- **C2: Részletesebb Képelemzési Tippek:** Konkrétabb javaslatok a fotók javítására.

## Won't-have (Nem része az MVP-nek):

- **W1: Natív mobilalkalmazások (iOS/Android).**

- **W2: Automatikus közzététel külső ingatlanportálokra.**

- **W3: "Feladatok és Naptár" komponens.**

- **W4: Részletes B2B funkciók kis ingatlanirodáknak** (pl. white-label API, tömeges feltöltés – ezek hosszabb távú tervek).

- **W5: Haladó MI funkciók** (pl. alaprajz generálás vázlatból, home staging MI, A/B tesztelő hirdetésoptimalizáló).

---

# PRD - Felhasználói Interakció és Tervezés 

  
Ez a rész vázolja a fő felhasználói folyamatokat, képernyőkoncepciókat és alapvető UI/UX tervezési elveket a Duumbi MVP számára.
## Belépési Pont

A felhasználók a Duumbi nyilvános portáljára érkeznek. Itt alapvető információkat találnak a szolgáltatásról, és lehetőségük lesz ingyenesen, regisztráció nélkül ingatlant keresni (korlátozott funkcionalitással).
## Regisztráció/Bejelentkezés

A prémium hirdetéslétrehozási funkciók és a részletesebb keresési lehetőségek eléréséhez regisztrációra és későbbi bejelentkezésre lesz szükség.

## Főképernyő

- Tiszta, modern dizájn.

- Jól látható keresősáv.

- Egyértelmű navigáció a hirdetéslétrehozás és a részletes keresés felé (regisztráció után).

- Információk a Duumbi előnyeiről és csomagjairól.

## MI Asszisztens Interakció

- **Hirdetéslétrehozás:** Csevegésszerű vagy kérdőíves, vezetett folyamat, ahol az MI kérdéseket tesz fel és a válaszok alapján tartalmat generál.

- **Keresés:** Az MI segítség az intelligens szűrőkön keresztül nyilvánul meg.

## UI/UX Alapelvek

- **Egyszerűség és Intuitivitás:** Könnyen érthető és használható felületek.

- **Konzisztencia:** Egységes dizájnelemek és működési logika a platformon keresztül.

- **Vizuális Hierarchia:** Világosan megkülönböztethető és értelmezhető tartalmi egységek.

- **Visszajelzés:** Azonnali és egyértelmű visszajelzés a felhasználói interakciókra.

- **Reszponzivitás:** A webes felület biztosítja az optimális megjelenést és használhatóságot asztali és mobil eszközökön egyaránt.

---
  
# PRD - Feltételezések

A Duumbi MVP fejlesztése és stratégiája a következő kulcsfontosságú feltételezésekre épül:

- **Piaci Igény:** Valós és kielégítetlen kereslet van egy MI-támogatott, felhasználóbarát ingatlanhirdetési és -kereső platform iránt a célcsoportok (magánhirdetők, keresők) körében.

- **Technológiai Hozzáférés:** A célfelhasználók rendelkeznek a szükséges eszközökkel és stabil internetkapcsolattal a webalkalmazás használatához.

- **Adatforrások:** Az egyszerűsített árajánlásokhoz és piaci kontextushoz szükséges alapvető adatok (pl. átlagárak) rendelkezésre állnak vagy becsülhetők az MVP számára.

- **Felhasználói Elfogadás:** A felhasználók nyitottak az MI-alapú eszközök használatára ingatlanügyeik intézésében, és megbíznak a platform által nyújtott javaslatokban.

- **Adatbiztonság és Adatvédelem:** A felhasználók bizalmát átlátható és GDPR-kompatibilis adatkezelési gyakorlatokkal nyerjük el.

---

# PRD - Jövőbeli Megfontolások

Az MVP sikeres elindítása és validálása után a Duumbi platform továbbfejleszthető a Duumbi Üzleti Tervében vázolt ütemterv alapján, beleértve, de nem kizárólagosan:

## Fejlettebb MI Funkciók:

- Részletes képelemzés és javítási javaslatok.

- Alaprajz generálás kézzel rajzolt vázlatokból vagy fotókból.

- Home staging MI (virtuális berendezési tippek).

- Hirdetésoptimalizáló modul (A/B tesztelés).

- Fejlettebb, chatbotszerű interakciók az MI asszisztenssel.

## B2B Funkciók Bővítése:

- Dedikált eszközök és API kis ingatlanirodák számára.

- Tömeges feltöltési lehetőségek.

## Nemzetközi Terjeszkedés:

- Új nyelvek és piacok támogatása (DACH régió, Spanyolország, az üzleti terv szerint).

## Partneri Integrációk:

- Szorosabb együttműködés más ingatlanpiaci szereplőkkel és szolgáltatókkal.