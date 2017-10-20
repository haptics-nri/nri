/////////////////////////////////////////////////////////////////////////////
// Name:        thingdef.h
// Purpose:     Definitions for wxThings
// Author:      John Labenski
// Modified by:
// Created:     1/08/1999
// RCS-ID:      $Id: thingdef.h,v 1.2 2005-09-20 14:33:45 hg Exp $
// Copyright:   (c) John Labenski
// Licence:     wxWidgets licence
/////////////////////////////////////////////////////////////////////////////

#ifndef __WX_THINGDEF_H__
#define __WX_THINGDEF_H__

//#if defined(__GNUG__) && !defined(NO_GCC_PRAGMA)
//    #pragma interface "thingdef.h"
//#endif

#include "wx/defs.h"
//#include "wxthings/wx24defs.h"   // wx2.4 backwards compatibility

// ----------------------------------------------------------------------------
// DLLIMPEXP macros
// ----------------------------------------------------------------------------

// These are our DLL macros (see the contrib libs like wxPlot)
#ifdef WXMAKINGDLL_THINGS
#define WXDLLIMPEXP_THINGS WXEXPORT
#define WXDLLIMPEXP_DATA_THINGS(type) WXEXPORT type
#elif defined(WXUSINGDLL)
#define WXDLLIMPEXP_THINGS WXIMPORT
#define WXDLLIMPEXP_DATA_THINGS(type) WXIMPORT type
#else // not making nor using DLL
#define WXDLLIMPEXP_THINGS
#define WXDLLIMPEXP_DATA_THINGS(type) type
#endif

#endif  // __WX_THINGDEF_H__

