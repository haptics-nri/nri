//-----------------------------------------------------------------------------
#include "CustomValidators.h"

//=============================================================================
//================= Implementation IPv4StringValidator ==========================
//=============================================================================
//-----------------------------------------------------------------------------
IPv4StringValidator::IPv4StringValidator( wxString* valPtr /* = NULL */ ) : wxTextValidator( wxFILTER_INCLUDE_CHAR_LIST, valPtr )
//-----------------------------------------------------------------------------
{
    wxArrayString strings;
    strings.push_back( wxT( "." ) );
    for( unsigned int i = 0; i <= 9; i++ )
    {
        wxString s;
        s << i;
        strings.push_back( s );
    }
    SetIncludes( strings );
}

//=============================================================================
//================= Implementation MACStringValidator =========================
//=============================================================================
//-----------------------------------------------------------------------------
MACStringValidator::MACStringValidator( wxString* valPtr /* = NULL */ ) : wxTextValidator( wxFILTER_INCLUDE_CHAR_LIST, valPtr )
//-----------------------------------------------------------------------------
{
    wxArrayString strings;
    strings.push_back( wxT( "a" ) );
    strings.push_back( wxT( "b" ) );
    strings.push_back( wxT( "c" ) );
    strings.push_back( wxT( "d" ) );
    strings.push_back( wxT( "e" ) );
    strings.push_back( wxT( "f" ) );
    strings.push_back( wxT( "A" ) );
    strings.push_back( wxT( "B" ) );
    strings.push_back( wxT( "C" ) );
    strings.push_back( wxT( "D" ) );
    strings.push_back( wxT( "E" ) );
    strings.push_back( wxT( "F" ) );
    for( unsigned int i = 0; i <= 9; i++ )
    {
        wxString s;
        s << i;
        strings.push_back( s );
    }
    SetIncludes( strings );
}
