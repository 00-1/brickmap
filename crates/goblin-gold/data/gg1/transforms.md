# GG1 transforms — pure `datum → { p, a }` functions

Each mode builds its round as `shuffle(<POOL>).map(<transform>)`. `shuffle` randomises ORDER
only (Fisher–Yates) — it is NOT part of the content contract; the parity vectors are the sorted,
order-independent set. Every transform below is a **pure** function of one pool datum `e`,
returning `{ p: <prompt string>, a: <numpad answer> }`. A port reproduces GG1 by applying these
to `pool` and checking the result against `parity-vectors.json`.

## `addSubItem`  — used by: addsub, addsub2
```js
function addSubItem(e){
    const a = e[0], b = e[1];
    return e[2] ? { p: a + " " + MINUS + " " + b, a: a - b }
                : { p: a + " + " + b, a: a + b };
  }
```

## `bondP1Item`  — used by: bonds
```js
function bondP1Item(x){ return { p: x + " + ? = 100", a: 100 - x }; }
```

## `bondP2Item`  — used by: bonds2
```js
function bondP2Item(e){ return { p: e[0] + " + ? = " + e[1], a: e[2] }; }
```

## `pvItem`  — used by: placevalue, placevalue2
```js
function pvItem(e){ return { p: e[0] + " " + e[1] + " " + e[2], a: e[3] }; }
```

## `fractionsOfItem`  — used by: fractionsof, fractionsof2
```js
function fractionsOfItem(e){ return { p: e[0] + "/" + e[1] + " of " + e[2], a: e[3] }; }
```

## `percentItem`  — used by: percentages, percentages2
```js
function percentItem(e){ return { p: e[0] + "% of " + e[1], a: e[2] }; }
```

## `roundingItem`  — used by: rounding
```js
function roundingItem(e){ return { p: e[0] + " to nearest " + e[1], a: e[2] }; }
```

## `largerMdItem`  — used by: largermd
```js
function largerMdItem(e){ return { p: e[0] + " " + e[1] + " " + e[2], a: e[3] }; }
```

## `metricItem`  — used by: metric
```js
function metricItem(e){ return { p: e[0] + " " + e[1] + " in " + e[2], a: e[3] }; }
```

## `sequenceItem`  — used by: sequences, sequences2
```js
function sequenceItem(e){
    if(e[0] === "next") return { p: "next: " + e[1].join(", "), a: e[2] };
    const add = e[2];
    const rule = e[1] + "n" + (add < 0 ? " − " + (-add) : add > 0 ? " + " + add : "");
    return { p: rule + ", term " + e[3], a: e[4] };
  }
```

## `scalingItem`  — used by: scaling
```js
function scalingItem(e){ return { p: e[0] + "→" + e[1] + ", " + e[2] + "→?", a: e[3] }; }
```

## `percentOffItem`  — used by: percentoff
```js
function percentOffItem(e){ return { p: e[0] + "% off " + e[1], a: e[2] }; }
```

## `partWholeItem`  — used by: partwhole
```js
function partWholeItem(e){
    if(e[0] === "f") return { p: e[1] + "/" + e[2] + " of ? = " + e[3], a: e[4] };
    return { p: e[1] + "% of ? = " + e[2], a: e[3] };
  }
```

## `balanceItem`  — used by: balance
```js
function balanceItem(e){ return { p: e[0] + " " + e[1] + " " + e[2] + " = " + e[3] + " " + e[4] + " ?", a: e[5] }; }
```

## `lcmhcfItem`  — used by: lcmhcf
```js
function lcmhcfItem(e){ return { p: e[0] + " " + e[1] + "," + e[2], a: e[3] }; }
```

## `meanItem`  — used by: mean
```js
function meanItem(e){
    if(e[0] === "f") return { p: "mean of " + e[1].join(","), a: e[2] };
    return { p: "mean of " + e[1].join(",") + ",? is " + e[2], a: e[3] };
  }
```

## `timegapItem`  — used by: timegap
```js
function timegapItem(e){ const z = n => (n < 10 ? "0" : "") + n; return { p: z(e[0]) + ":" + z(e[1]) + " → " + z(e[2]) + ":" + z(e[3]), a: e[4] }; }
```

## `ratioShareItem`  — used by: ratioshare
```js
function ratioShareItem(e){
    if(e[0] === "2") return { p: e[1] + " in " + e[2] + ":" + e[3] + " → " + (e[4] === "big" ? "bigger" : "smaller"), a: e[5] };
    return { p: e[1] + " in " + e[2] + ":" + e[3] + ":" + e[4] + " → biggest", a: e[6] };
  }
```

## `cubeRootItem`  — used by: cubes
```js
function cubeRootItem(e){
    if(e[0] === "c") return { p: e[1] + "³", a: e[1] * e[1] * e[1] };
    return { p: e[0] + e[1], a: e[2] };
  }
```

## `moneyItem`  — used by: money
```js
function moneyItem(e){
    if(e[0] === "m") return { p: e[1] + " × £" + e[2].toFixed(2), a: e[3] };
    return { p: "change from £" + e[1] + " of £" + e[2].toFixed(2), a: e[3] };
  }
```

## `digitsumItem`  — used by: digitsum
```js
function digitsumItem(e){
    if(e[0] === "s") return { p: "digit sum of " + e[1], a: e[2] };
    return { p: "remainder " + e[1] + " ÷ 9", a: e[2] };
  }
```

## `romanItem`  — used by: roman
```js
function romanItem(e){ return { p: e[0], a: e[1] }; }
```

## `primeItem`  — used by: primes
```js
function primeItem(e){ return { p: "next prime > " + e[0], a: e[1] }; }
```

## `pctUpItem`  — used by: pctup
```js
function pctUpItem(e){ return { p: e[1] + " + " + e[0] + "%", a: e[2] }; }
```

## `fdpItem`  — used by: fdp
```js
function fdpItem(e){
    if(e[0] === "d") return { p: e[1] + "% as a decimal", a: e[2] };
    if(e[0] === "p") return { p: e[1] + " as a %", a: e[2] };
    return { p: e[1] + "/" + e[2] + " as a %", a: e[3] };
  }
```

## `exprItem`  — used by: bodmas, algebra, negatives
```js
function exprItem(e){ return { p: e[0], a: e[1] }; }
```

## `xtrickItem`  — used by: xtricks
```js
function xtrickItem(e){ return { p: e[0] + " × " + e[1], a: e[2] }; }
```

## `areaItem`  — used by: area
```js
function areaItem(e){
    if(e[0] === "ar") return { p: "area " + e[1] + "×" + e[2], a: e[3] };
    if(e[0] === "pr") return { p: "perim " + e[1] + "×" + e[2], a: e[3] };
    return { p: "△ " + e[1] + "×" + e[2], a: e[3] };
  }
```

## `volItem`  — used by: volume
```js
function volItem(e){ return { p: "vol " + e[0] + "×" + e[1] + "×" + e[2], a: e[3] }; }
```

## `angleItem`  — used by: angles
```js
function angleItem(e){
    if(e[0] === "L") return { p: "line " + e[1] + " + ?", a: e[2] };
    if(e[0] === "P") return { p: "point " + e[1] + " + ?", a: e[2] };
    return { p: "△ " + e[1] + ", " + e[2] + " → ?", a: e[3] };
  }
```

## `mmrItem`  — used by: mmr
```js
function mmrItem(e){ const label = e[0] === "med" ? "median" : e[0] === "mod" ? "mode" : "range"; return { p: label + " of " + e[1].join(","), a: e[2] }; }
```

## `sdtItem`  — used by: sdt
```js
function sdtItem(e){
    if(e[0] === "d") return { p: "dist: " + e[1] + "km/h × " + e[2] + "h", a: e[3] };
    if(e[0] === "s") return { p: "speed: " + e[1] + "km in " + e[2] + "h", a: e[3] };
    return { p: "time: " + e[1] + "km at " + e[2] + "km/h", a: e[3] };
  }
```

## `factorsItem`  — used by: factors
```js
function factorsItem(e){
    if(e[0] === "nf") return { p: "# factors of " + e[1], a: e[2] };
    if(e[0] === "nm") return { p: "next ×" + e[1] + " > " + e[2], a: e[3] };
    return { p: "biggest prime of " + e[1], a: e[2] };
  }
```

## Inline transforms (no named fn)

- `halves`: `n => ({ p:String(n), a:n/2 })`
- `times`: `([a,b]) => ({ p:a+" × "+b, a:a*b })`
- `doubles`: `n => ({ p:String(n), a:n*2 })`
- `fractions`: `([f,d]) => ({ p:f, a:d })`
- `fractions2`: `([f,d]) => ({ p:f, a:d })`
- `squares`: `n => ({ p:n+"²", a:n*n })`

## Shared constants (part of the content contract — reproduce these)

- `MINUS` = `"−"`

## Shared helper (order only — do NOT port as content)

```js
function shuffle(a){
    a = a.slice();
    for(let i=a.length-1;i>0;i--){ const j=Math.floor(Math.random()*(i+1)); [a[i],a[j]]=[a[j],a[i]]; }
    return a;
  }
```
