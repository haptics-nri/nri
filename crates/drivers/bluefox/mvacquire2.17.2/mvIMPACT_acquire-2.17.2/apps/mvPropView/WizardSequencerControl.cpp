#include <algorithm>
#include <apps/Common/wxAbstraction.h>
#include <common/STLHelper.h>
#include "PropViewFrame.h"
#include "WizardSequencerControl.h"
#include <wx/combobox.h>
#include <wx/spinctrl.h>

using namespace std;

//=============================================================================
//============== Implementation WizardSequencerControl ========================
//=============================================================================
//-----------------------------------------------------------------------------
WizardSequencerControl::WizardSequencerControl( wxWindow* pParent, const wxString& title, mvIMPACT::acquire::Device* pDev, size_t displayCount, const vector<long>& setToDisplayTable )
    : OkAndCancelDlg( pParent, widMainFrame, title, wxDefaultPosition, wxDefaultSize, wxDEFAULT_DIALOG_STYLE | wxMINIMIZE_BOX | wxRESIZE_BORDER ),
      sequencerSetGUIData_(), sequencerSetNextTable_(), setToDisplayTable_( setToDisplayTable ), displayCoices_(), boGUICreated_( false ), sc_( pDev )
//-----------------------------------------------------------------------------
{
    /*
        |-------------------------------------|
        | pTopDownSizer                       |
        |                spacer               |
        | |---------------------------------| |
        | | sequencer control controls      | |
        | |---------------------------------| |
        |                spacer               |
        | |---------------------------------| |
        | | pButtonSizer                    | |
        | |---------------------------------| |
        |-------------------------------------|
    */

    for( size_t i = 0; i < displayCount; i++ )
    {
        displayCoices_.Add( wxString::Format( wxT( "Display %d" ), static_cast<int>( i ) ) );
    }

    wxScrolledWindow* pPanel = new wxScrolledWindow( this );
    pPanel->SetScrollRate( 10, 10 );
    wxBoxSizer* pTopDownSizer = new wxBoxSizer( wxVERTICAL );
    pTopDownSizer->AddSpacer( 10 );

    int flexGridColumns = 3;
    vector<string> selectableFeaturesForSequencer;
    if( sc_.sequencerFeatureSelector.isValid() && sc_.sequencerSetLoad.isValid() )
    {
        sc_.sequencerFeatureSelector.getTranslationDictStrings( selectableFeaturesForSequencer );
        const vector<string>::size_type selectableFeaturesForSequencerCount = selectableFeaturesForSequencer.size();
        ComponentLocator locator( sc_.sequencerSetSelector.parent().parent() );
        for( vector<string>::size_type i = 0; i < selectableFeaturesForSequencerCount; i++ )
        {
            HOBJ hObj = locator.findComponent( selectableFeaturesForSequencer[i] );
            if( hObj != INVALID_ID )
            {
                sequenceableFeatures_.push_back( hObj );
            }
        }
    }
    const vector<string>::size_type sequenceableFeaturesCount = sequenceableFeatures_.size();
    wxFlexGridSizer* pSequencerControlControlsSizer = new wxFlexGridSizer( flexGridColumns + static_cast<int>( sequenceableFeaturesCount ), 5, 10 );
    pSequencerControlControlsSizer->Add( new wxStaticText( pPanel, wxID_ANY, wxT( "Set: " ) ) );
    pSequencerControlControlsSizer->Add( new wxStaticText( pPanel, wxID_ANY, wxT( "Next Set: " ) ) );
    ComponentLocator locator( sc_.sequencerSetSelector.parent().parent() );
    for( vector<HOBJ>::size_type i = 0; i < sequenceableFeaturesCount; i++ )
    {
        Component c( sequenceableFeatures_[i] );
        pSequencerControlControlsSizer->Add( new wxStaticText( pPanel, wxID_ANY, wxString::Format( wxT( "%s: " ), ConvertedString( c.name() ).c_str() ) ) );
    }
    pSequencerControlControlsSizer->Add( new wxStaticText( pPanel, wxID_ANY, wxT( "Display To Use: " ) ) );
    const int64_type sequencerSetCount = sc_.sequencerSetSelector.getMaxValue() + 1;
    if( setToDisplayTable_.size() > static_cast<std::vector<long>::size_type>( sequencerSetCount ) )
    {
        setToDisplayTable_.resize( sequencerSetCount );
    }
    for( int64_type i = 0; i < sequencerSetCount; i++ )
    {
        CreateSequencerSetControls( i, pPanel, pSequencerControlControlsSizer );
    }
    pTopDownSizer->Add( pSequencerControlControlsSizer, wxSizerFlags( 2 ).Expand() );
    pTopDownSizer->AddSpacer( 10 );
    AddButtons( pPanel, pTopDownSizer, false );

    wxBoxSizer* pOuterSizer = new wxBoxSizer( wxHORIZONTAL );
    pOuterSizer->AddSpacer( 5 );
    pOuterSizer->Add( pTopDownSizer, wxSizerFlags().Expand() );
    pOuterSizer->AddSpacer( 5 );

    pPanel->SetSizer( pOuterSizer );
    SetClientSize( pOuterSizer->GetMinSize() );
    wxSize maxSize( GetSize() );
    maxSize.SetHeight( maxSize.GetHeight() + wxSystemSettings::GetMetric( wxSYS_HSCROLL_Y ) );
    SetMaxSize( maxSize );
    wxSize minSize( GetSize() );
    minSize.SetHeight( -1 );
    SetMinSize( minSize );
    boGUICreated_ = true;
}

//-----------------------------------------------------------------------------
WizardSequencerControl::~WizardSequencerControl()
//-----------------------------------------------------------------------------
{
    for_each( sequencerSetGUIData_.begin(), sequencerSetGUIData_.end(), ptr_fun( DeleteElement<SequencerSetControls*> ) );
}

//-----------------------------------------------------------------------------
void WizardSequencerControl::CreateSequencerSetControls( int64_type index, wxPanel* pPanel, wxFlexGridSizer* pSequencerControlControlsSizer )
//-----------------------------------------------------------------------------
{
    SequencerSetControls* pData = new SequencerSetControls();
    pSequencerControlControlsSizer->Add( new wxStaticText( pPanel, wxID_ANY, wxString::Format( wxT( "Set %d: " ), static_cast<int>( index ) ) ) );
    try
    {
        sc_.sequencerSetSelector.write( index );
        const int sequencerSetNext = static_cast<int>( sc_.sequencerSetNext.read() );
        pData->pSequencerSetNext_ = new wxSpinCtrl( pPanel, wxID_ANY, wxString::Format( wxT( "%d" ), sequencerSetNext ), wxDefaultPosition, wxSize( 50, -1 ), wxSP_ARROW_KEYS,
                static_cast<int>( sc_.sequencerSetSelector.getMinValue() ),
                static_cast<int>( sc_.sequencerSetSelector.getMaxValue() ),
                sequencerSetNext );
        pSequencerControlControlsSizer->Add( pData->pSequencerSetNext_ );
        vector<HOBJ>::size_type selectableFeaturesForSequencerCount = sequenceableFeatures_.size();
        for( vector<HOBJ>::size_type i = 0; i < selectableFeaturesForSequencerCount; i++ )
        {
            sc_.sequencerSetLoad.call();
            Property prop( sequenceableFeatures_[i] );
            pSequencerControlControlsSizer->Add( new wxStaticText( pPanel, wxID_ANY, ConvertedString( prop.readS() ) ) );
        }
    }
    catch( const ImpactAcquireException& e )
    {
        dynamic_cast<PropViewFrame*>( GetParent() )->WriteErrorMessage( wxString::Format( wxT( "%s(%d): Internal error: %s(%s) while trying to access 'SequencerSetNext' property at set %d.\n" ), ConvertedString( __FUNCTION__ ).c_str(), __LINE__, ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str(), static_cast<int>( index ) ) );
    }
    wxArrayString::size_type displayIndex = 0;
    if( !setToDisplayTable_.empty() && ( setToDisplayTable_.size() > static_cast<std::vector<long>::size_type>( index ) ) )
    {
        if( ( setToDisplayTable_[index] >= 0 ) && ( setToDisplayTable_[index] <= static_cast<long>( displayCoices_.GetCount() - 1 ) ) )
        {
            displayIndex = static_cast<wxArrayString::size_type>( setToDisplayTable_[index] );
        }
    }
    pData->pDisplayToUse_ = new wxComboBox( pPanel, wxID_ANY, displayCoices_[displayIndex], wxDefaultPosition, wxDefaultSize, displayCoices_, wxCB_DROPDOWN | wxCB_READONLY );
    pSequencerControlControlsSizer->Add( pData->pDisplayToUse_ );
    sequencerSetGUIData_.push_back( pData );
}

//-----------------------------------------------------------------------------
const vector<int64_type>& WizardSequencerControl::GetSequencerSetNextTable( void ) const
//-----------------------------------------------------------------------------
{
    sequencerSetNextTable_.clear();
    const vector<SequencerSetControls*>::size_type setCount = sequencerSetGUIData_.size();
    for( vector<SequencerSetControls*>::size_type i = 0; i < setCount; i++ )
    {
        sequencerSetNextTable_.push_back( static_cast<int64_type>( sequencerSetGUIData_[i]->pSequencerSetNext_->GetValue() ) );
    }
    return sequencerSetNextTable_;
}

//-----------------------------------------------------------------------------
const vector<long>& WizardSequencerControl::GetSetToDisplayTable( void ) const
//-----------------------------------------------------------------------------
{
    setToDisplayTable_.clear();
    const vector<SequencerSetControls*>::size_type setCount = sequencerSetGUIData_.size();
    for( vector<SequencerSetControls*>::size_type i = 0; i < setCount; i++ )
    {
        const wxString selection( sequencerSetGUIData_[i]->pDisplayToUse_->GetValue().AfterLast( wxT( ' ' ) ) );
        long index = 0;
        if( selection.ToLong( &index ) )
        {
            setToDisplayTable_.push_back( index );
        }
        else
        {
            setToDisplayTable_.push_back( 0 );
        }
    }
    return setToDisplayTable_;
}
