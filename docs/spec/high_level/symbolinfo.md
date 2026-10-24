# SymbolInfo

`SymbolInfo` describes visible function symbols.

Example:

```n
@info >> SymbolInfo : symbolInfo(@fn(run));
return functionName(@info) == "run";
```

Private functions are rejected with a visibility diagnostic. Unknown functions are rejected with a symbol metadata diagnostic. Symbol metadata is observational only and cannot invoke or mutate the symbol.
