//-----------------------------------------------------------------------------
#ifndef LineProfileCanvasHorizontalH
#define LineProfileCanvasHorizontalH LineProfileCanvasHorizontalH
//-----------------------------------------------------------------------------
#include "LineProfileCanvas.h"

//-----------------------------------------------------------------------------
class LineProfileCanvasHorizontal : public LineProfileCanvas
//-----------------------------------------------------------------------------
{
    void                    ProcessRGB_8u_CxData( const ImageBuffer* pIB, const int inc, const int order[3] );
    template<typename _Ty>
    void                    ProcessMonoPackedData( const ImageBuffer* pIB, const TBayerMosaicParity bayerParity, _Ty pixelAccessFn );
    template<typename _Ty>
    void                    ProcessYUVData( const ImageBuffer* pIB );
    template<typename _Ty>
    void                    ProcessYUV444Data( const ImageBuffer* pIB );
    template<typename _Ty>
    void                    ProcessUYVData( const ImageBuffer* pIB );
protected:
    virtual void            CalculateData( const ImageBuffer* pIB, const TBayerMosaicParity bayerParity );
    virtual int             GetDataCount( void ) const
    {
        return m_horDataCount;
    }
    virtual unsigned int    GetXMarkerParameters( unsigned int& from, unsigned int& to ) const
    {
        from = m_AOIx;
        to = m_AOIw + m_AOIx;
        return GetXMarkerStepWidth( from, to );
    }
public:
    explicit                LineProfileCanvasHorizontal() {}
    explicit                LineProfileCanvasHorizontal( wxWindow* parent, wxWindowID id = -1, const wxPoint& pos = wxDefaultPosition,
            const wxSize& size = wxDefaultSize, long style = wxSUNKEN_BORDER,
            const wxString& name = wxT( "Horizontal Line Profile" ), bool boActive = false );
};

#endif // LineProfileCanvasHorizontalH
