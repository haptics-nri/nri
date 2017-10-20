//-----------------------------------------------------------------------------
#ifndef CustomValidatorsH
#define CustomValidatorsH CustomValidatorsH
//-----------------------------------------------------------------------------
#include "wx/wx.h"

//-----------------------------------------------------------------------------
class IPv4StringValidator : public wxTextValidator
//-----------------------------------------------------------------------------
{
public:
    IPv4StringValidator( wxString* valPtr = NULL );
};

//-----------------------------------------------------------------------------
class MACStringValidator : public wxTextValidator
//-----------------------------------------------------------------------------
{
public:
    MACStringValidator( wxString* valPtr = NULL );
};

#endif // CustomValidatorsH
