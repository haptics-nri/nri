//-----------------------------------------------------------------------------
#ifndef expatHelperH
#define expatHelperH expatHelperH
//-----------------------------------------------------------------------------
#include <memory.h>
#include <stdio.h>
#include <Toolkits/expat/include/ExpatImpl.h>

//-----------------------------------------------------------------------------
template<class _Ty>
bool ParseXMLFromFile( CExpatImpl<_Ty>& parser, FILE* fp )
//-----------------------------------------------------------------------------
{
    if( !fp )
    {
        return false;
    }

    fseek( fp, 0, SEEK_END );
    const int fileSize = ftell( fp );
    fseek( fp, 0, SEEK_SET );

    if( fileSize <= 0 )
    {
        return false;
    }

    char* pszBuffer = reinterpret_cast<char*>( parser.GetBuffer( fileSize  + 1 ) );
    if( !pszBuffer )
    {
        return false;
    }

    const size_t bytesRead = fread( pszBuffer, 1, static_cast<size_t>( fileSize ), fp );
    pszBuffer[bytesRead] = '\0'; // make this a well terminated string!
    return parser.ParseBuffer( static_cast<int>( bytesRead ), true );
}

//-----------------------------------------------------------------------------
template<class _Ty>
bool ParseXMLFromBuffer( CExpatImpl<_Ty>& parser, const char* pBuf, const size_t bufSize )
//-----------------------------------------------------------------------------
{
    char* pszBuffer = reinterpret_cast<char*>( parser.GetBuffer( static_cast<int>( bufSize ) + 1 ) );
    if( !pszBuffer )
    {
        return false;
    }

    memcpy( pszBuffer, pBuf, bufSize );
    pszBuffer[bufSize] = '\0'; // make this a well terminated string!
    return parser.ParseBuffer( static_cast<int>( bufSize ), true );
}

#endif // expatHelperH
