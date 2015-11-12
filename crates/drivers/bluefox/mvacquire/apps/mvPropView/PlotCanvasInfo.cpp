#include <apps/Common/wxAbstraction.h>
#include <algorithm>
#include <limits>
#include "PlotCanvasInfo.h"
#include <wx/dcbuffer.h>

using namespace std;
using namespace mvIMPACT::acquire;

//=============================================================================
//============== Implementation PlotCanvasInfo ================================
//=============================================================================
//-----------------------------------------------------------------------------
PlotCanvasInfo::PlotCanvasInfo( wxWindow* parent, wxWindowID id /* = -1 */, const wxPoint& pos /* = wxDefaultPosition */,
                                const wxSize& size /* = wxDefaultSize */, long style /* = wxSUNKEN_BORDER */,
                                const wxString& name /* = "info plot" */, bool boActive /* = false */ )
    : PlotCanvas( parent, id, pos, size, style, name, boActive ),
      m_HistoryDepth( 20 ), m_ppPlotValues( 0 ), m_CurrentMaxPlotValues(), m_CurrentMinPlotValues(), m_CurrentPlotValues(),
      m_CurrentMaxPlotValue( numeric_limits<plot_data_type>::min() ), m_CurrentMinPlotValue( numeric_limits<plot_data_type>::max() ),
      m_boPlotDifferences( false )
//-----------------------------------------------------------------------------
{
    AllocateDataBuffer( 1 );
    m_pens[0] = *wxRED;
    m_pens[1] = *wxGREEN;
    m_pens[2] = *wxBLUE;
    m_pens[3] = wxColour( 128, 128, 0 );
}

//-----------------------------------------------------------------------------
PlotCanvasInfo::~PlotCanvasInfo()
//-----------------------------------------------------------------------------
{
    DeallocateDataBuffer();
}

//-----------------------------------------------------------------------------
void PlotCanvasInfo::AllocateDataBuffer( unsigned int plotCount )
//-----------------------------------------------------------------------------
{
    DeallocateDataBuffer();
    m_PlotCount = plotCount;
    m_ppPlotValues = new std::deque<plot_data_type>* [m_PlotCount];
    m_CurrentMaxPlotValues.resize( m_PlotCount );
    m_CurrentMinPlotValues.resize( m_PlotCount );
    m_CurrentPlotValues.resize( m_PlotCount );
    m_PlotIdentifiers.resize( m_PlotCount );
    for( unsigned int i = 0; i < m_PlotCount; i++ )
    {
        m_ppPlotValues[i] = new std::deque<plot_data_type>();
        m_CurrentMaxPlotValues[i] = numeric_limits<plot_data_type>::min();
        m_CurrentMinPlotValues[i] = numeric_limits<plot_data_type>::max();
        m_CurrentPlotValues[i] = 0;
    }
}

//-----------------------------------------------------------------------------
void PlotCanvasInfo::ClearCache( void )
//-----------------------------------------------------------------------------
{
    wxCriticalSectionLocker locker( m_critSect );
    ClearCache_Internal();
}

//-----------------------------------------------------------------------------
void PlotCanvasInfo::ClearCache_Internal( void )
//-----------------------------------------------------------------------------
{
    for( unsigned int i = 0; i < m_PlotCount; i++ )
    {
        m_ppPlotValues[i]->clear();
        m_CurrentMaxPlotValues[i] = numeric_limits<plot_data_type>::min();
        m_CurrentMinPlotValues[i] = numeric_limits<plot_data_type>::max();
    }
    m_CurrentMaxPlotValue = numeric_limits<plot_data_type>::min();
    m_CurrentMinPlotValue = numeric_limits<plot_data_type>::max();
    Refresh( true );
}

//-----------------------------------------------------------------------------
void PlotCanvasInfo::DeallocateDataBuffer( void )
//-----------------------------------------------------------------------------
{
    if( m_ppPlotValues )
    {
        for( unsigned int i = 0; i < m_PlotCount; i++ )
        {
            delete m_ppPlotValues[i];
        }
        delete [] m_ppPlotValues;
        m_ppPlotValues = 0;
        m_CurrentMaxPlotValues.clear();
        m_CurrentMinPlotValues.clear();
        m_CurrentPlotValues.clear();
        m_PlotIdentifiers.clear();
        m_PlotCount = 0;
    }
}

//-----------------------------------------------------------------------------
double PlotCanvasInfo::GetScaleX( wxCoord w ) const
//-----------------------------------------------------------------------------
{
    return static_cast<double>( w - 2 * GetBorderWidth() ) / static_cast<double>( m_HistoryDepth );
}

//-----------------------------------------------------------------------------
double PlotCanvasInfo::GetScaleY( wxCoord h ) const
//-----------------------------------------------------------------------------
{
    const int64_type offset = ( m_CurrentMinPlotValue < 0 ) ? m_CurrentMinPlotValue : 0;
    const int64_type currentYRange = m_CurrentMaxPlotValue - offset;
    return static_cast<double>( h - 2 * GetBorderWidth() ) / static_cast<double>( ( currentYRange != 0 ) ? currentYRange : 1 );
}

//-----------------------------------------------------------------------------
unsigned int PlotCanvasInfo::GetXMarkerParameters( unsigned int& from, unsigned int& to ) const
//-----------------------------------------------------------------------------
{
    from = 0;
    to = m_HistoryDepth;
    return m_HistoryDepth / 5;
}

//-----------------------------------------------------------------------------
void PlotCanvasInfo::OnPaintCustom( wxPaintDC& dc )
//-----------------------------------------------------------------------------
{
    wxCoord xOffset( 1 ), w( 0 ), h( 0 );
    dc.GetSize( &w, &h );
    const double scaleX = GetScaleX( w );
    const double scaleY = GetScaleY( h );
    DrawMarkerLines( dc, w, h, scaleX );
    const int64_type offset = ( m_CurrentMinPlotValue < 0 ) ? m_CurrentMinPlotValue : 0;
    for( unsigned int i = 0; i < m_PlotCount; i++ )
    {
        const unsigned int valCount = static_cast<unsigned int>( m_ppPlotValues[i]->size() );
        if( valCount > 1 )
        {
            int lowerStart = h - GetBorderWidth();
            dc.SetPen( m_pens[i % COLOUR_COUNT] );
            for( unsigned int j = 0; j < valCount - 1; j++ )
            {
                dc.DrawLine( static_cast<int>( GetBorderWidth() + ( j * scaleX ) + 1 ),
                             static_cast<int>( lowerStart - ( ( ( *m_ppPlotValues[i] )[j] - offset ) * scaleY ) ),
                             static_cast<int>( GetBorderWidth() + ( ( j + 1 ) * scaleX ) + 1 ),
                             static_cast<int>( lowerStart - ( ( ( *m_ppPlotValues[i] )[j + 1] - offset ) * scaleY ) ) );
            }
            dc.SetPen( wxNullPen );
        }
        // info in the top left corner
        DrawInfoString( dc, wxString::Format( wxT( "%s: max: %8lld, min: %8lld: " ), m_PlotIdentifiers[i].c_str(), m_CurrentMaxPlotValues[i], m_CurrentMinPlotValues[i] ), xOffset, 1, m_pens[i] );
    }
}

//-----------------------------------------------------------------------------
bool PlotCanvasInfo::MustUpdate( bool boForceRefresh )
//-----------------------------------------------------------------------------
{
    if( ( ( m_ImageCount++ % m_UpdateFrequency ) == 0 ) || boForceRefresh )
    {
        return true;
    }
    return false;
}

//-----------------------------------------------------------------------------
void PlotCanvasInfo::RefreshData( const RequestInfoData& infoData, bool boForceRefresh /* = false */ )
//-----------------------------------------------------------------------------
{
    wxCriticalSectionLocker locker( m_critSect );
    if( !IsActive() )
    {
        return;
    }

    PlotHashTable::const_iterator it = m_PlotHashTable.find( infoData.settingUsed );
    unsigned int index = ( it == m_PlotHashTable.end() ) ? 0 : it->second;

    plot_data_type value;
    switch( infoData.plotValue.type )
    {
    case ctPropInt:
        value = static_cast<plot_data_type>( infoData.plotValue.value.intRep   );
        break;
    case ctPropInt64:
        value = static_cast<plot_data_type>( infoData.plotValue.value.int64Rep );
        break;
    case ctPropFloat:
        value = static_cast<plot_data_type>( infoData.plotValue.value.doubleRep );
        break;
    default:
        return;
    }

    m_ppPlotValues[index]->push_back( m_boPlotDifferences ? value - m_CurrentPlotValues[index] : value );
    m_CurrentPlotValues[index] = value;

    bool boFullUpdate = RefreshPlotData();

    if( !MustUpdate( boForceRefresh ) )
    {
        return;
    }

    Refresh( boFullUpdate );
}

//-----------------------------------------------------------------------------
bool PlotCanvasInfo::RefreshPlotData( void )
//-----------------------------------------------------------------------------
{
    bool boFullUpdate = false;

    for( unsigned int i = 0; i < m_PlotCount; i++ )
    {
        bool boMaxRemovedFromQueue = false;
        bool boMinRemovedFromQueue = false;
        while( m_ppPlotValues[i]->size() > m_HistoryDepth )
        {
            if( m_ppPlotValues[i]->front() == m_CurrentMaxPlotValues[i] )
            {
                boMaxRemovedFromQueue = true;
            }
            if( m_ppPlotValues[i]->front() == m_CurrentMinPlotValues[i] )
            {
                boMinRemovedFromQueue = true;
            }
            m_ppPlotValues[i]->pop_front();
        }

        if( boMaxRemovedFromQueue )
        {
            m_CurrentMaxPlotValues[i] = *( max_element( m_ppPlotValues[i]->begin(), m_ppPlotValues[i]->end() ) );
        }
        else if( !m_ppPlotValues[i]->empty() && ( m_ppPlotValues[i]->back() > m_CurrentMaxPlotValues[i] ) )
        {
            m_CurrentMaxPlotValues[i] = m_ppPlotValues[i]->back();
        }

        if( boMinRemovedFromQueue )
        {
            m_CurrentMinPlotValues[i] = *( min_element( m_ppPlotValues[i]->begin(), m_ppPlotValues[i]->end() ) );
        }
        else if( !m_ppPlotValues[i]->empty() && ( m_ppPlotValues[i]->back() < m_CurrentMinPlotValues[i] ) )
        {
            m_CurrentMinPlotValues[i] = m_ppPlotValues[i]->back();
        }
    }

    int64_type maxVal = *( max_element( m_CurrentMaxPlotValues.begin(), m_CurrentMaxPlotValues.end() ) );
    if( m_CurrentMaxPlotValue != maxVal )
    {
        m_CurrentMaxPlotValue = maxVal;
        boFullUpdate = true;
    }

    int64_type minVal = *( min_element( m_CurrentMinPlotValues.begin(), m_CurrentMinPlotValues.end() ) );
    if( m_CurrentMinPlotValue != minVal )
    {
        m_CurrentMinPlotValue = minVal;
        boFullUpdate = true;
    }

    return boFullUpdate;
}

//-----------------------------------------------------------------------------
void PlotCanvasInfo::SetHistoryDepth( unsigned int historyDepth )
//-----------------------------------------------------------------------------
{
    wxCriticalSectionLocker locker( m_critSect );
    m_HistoryDepth = historyDepth;
    RefreshPlotData();
}

//-----------------------------------------------------------------------------
void PlotCanvasInfo::SetPlotDifferences( bool boActive )
//-----------------------------------------------------------------------------
{
    wxCriticalSectionLocker locker( m_critSect );
    if( m_boPlotDifferences != boActive )
    {
        m_boPlotDifferences = boActive;
        ClearCache_Internal();
        RefreshPlotData();
    }
}

//-----------------------------------------------------------------------------
void PlotCanvasInfo::SetupPlotIdentifiers( const vector<pair<string, int> >& plotIdentifiers )
//-----------------------------------------------------------------------------
{
    wxCriticalSectionLocker locker( m_critSect );
    const unsigned int plotCount = static_cast<unsigned int>( plotIdentifiers.size() );
    AllocateDataBuffer( plotCount );
    m_PlotHashTable.clear();
    for( unsigned int i = 0; i < plotCount; i++ )
    {
        m_PlotHashTable.insert( make_pair( plotIdentifiers[i].second, i ) );
        m_PlotIdentifiers[i] = ConvertedString( plotIdentifiers[i].first );
    }
    Refresh( true );
}
