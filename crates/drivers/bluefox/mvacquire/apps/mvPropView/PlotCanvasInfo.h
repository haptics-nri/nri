//-----------------------------------------------------------------------------
#ifndef PlotCanvasInfoH
#define PlotCanvasInfoH PlotCanvasInfoH
//-----------------------------------------------------------------------------
#include "DevData.h"
#include "PlotCanvas.h"
#include <deque>

//-----------------------------------------------------------------------------
class PlotCanvasInfo : public PlotCanvas
//-----------------------------------------------------------------------------
{
public:
    explicit                     PlotCanvasInfo() {}
    explicit                     PlotCanvasInfo( wxWindow* parent, wxWindowID id = -1, const wxPoint& pos = wxDefaultPosition,
            const wxSize& size = wxDefaultSize, long style = wxSUNKEN_BORDER,
            const wxString& name = wxT( "info plot" ), bool boActive = false );
    ~PlotCanvasInfo();
    void                         ClearCache( void );
    void                         SetHistoryDepth( unsigned int historyDepth );
    unsigned int                 GetHistoryDepth( void ) const
    {
        return m_HistoryDepth;
    }
    void                         SetPlotDifferences( bool boActive );
    void                         SetupPlotIdentifiers( const std::vector<std::pair<std::string, int> >& plotIdentifiers );
    virtual void                 RefreshData( const RequestInfoData& infoData, bool boForceRefresh = false );
protected:
    virtual double               GetScaleX( wxCoord w ) const;
    virtual double               GetScaleY( wxCoord h ) const;
    virtual unsigned int         GetXMarkerParameters( unsigned int& from, unsigned int& to ) const;
    virtual void                 OnPaintCustom( wxPaintDC& dc );
private:
    typedef int64_type           plot_data_type;
    typedef std::map<int, int>   PlotHashTable;
    enum
    {
        COLOUR_COUNT = 4
    };

    unsigned int                 m_HistoryDepth;
    std::deque<plot_data_type>** m_ppPlotValues;
    std::vector<plot_data_type>  m_CurrentMaxPlotValues;
    std::vector<plot_data_type>  m_CurrentMinPlotValues;
    std::vector<plot_data_type>  m_CurrentPlotValues;
    wxColour                     m_pens[COLOUR_COUNT];
    PlotHashTable                m_PlotHashTable;
    std::vector<wxString>        m_PlotIdentifiers;
    size_t                       m_PlotCount;
    int64_type                   m_CurrentMaxPlotValue;
    int64_type                   m_CurrentMinPlotValue;
    bool                         m_boPlotDifferences;

    void                         AllocateDataBuffer( unsigned int plotCount );
    void                         ClearCache_Internal( void );
    void                         DeallocateDataBuffer( void );
    bool                         MustUpdate( bool boForceRefresh );
    bool                         RefreshPlotData( void );
};

#endif // PlotCanvasInfoH
