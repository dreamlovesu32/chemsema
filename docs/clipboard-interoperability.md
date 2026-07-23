# Clipboard and Multi-window Interoperability

ChemSema restores editable structure whenever the clipboard contains a ChemSema fragment/document, CDXML, CDX, or a readable Office/ChemDraw embedded object. Images are the final fallback.

Paste priority is:

1. ChemSema fragment or the portable payload embedded in CF_HTML.
2. ChemSema document JSON, pasted as objects into the current document.
3. ChemDraw interchange, CDXML, or CDX recovered from OLE storage.
4. CDXML in Unicode text.
5. PNG or DIB/BMP image data.

Web copies write HTML containing the complete portable payload and plain-text CDXML for the selected objects. Desktop copies expose the private formats, CF_HTML, CDXML, Unicode text, and Office OLE/EMF. This makes copying work across document tabs, detached desktop windows, browser tabs, and Web/desktop boundaries.

Word/ChemDraw embedded objects are read through `OleGetClipboard`. `Embedded Object` and `Embed Source` storage are checked first; a `CONTENTS` stream beginning with the official `VjCD0100` header is decoded as CDX. A malformed higher-priority format must not block the next structured fallback.
