#include <apps/Common/wxAbstraction.h>
#include <common/STLHelper.h>
#include <math.h> // for 'pow'
#include "WizardQuickSetup.h"
#include <wx/slider.h>
#include <wx/tglbtn.h>
#include "WizardQuickSetupIcons.h"

using namespace std;

/// \todo Add handler for the top line of buttons
/// \todo Add missing controls
/// \todo Think about creating the wizard with a much larger default size to have larger sliders
/// \todo wxPropView should start with the left toolbar and the property grid disabled on first go / add checkbox to configure this behaviour

//=============================================================================
//============== Implementation WizardQuickSetup ==============================
//=============================================================================
BEGIN_EVENT_TABLE( WizardQuickSetup, OkAndCancelDlg )
    EVT_BUTTON( widBtnPresetColor, WizardQuickSetup::OnBtnPresetColor )
    EVT_BUTTON( widBtnPresetFactory, WizardQuickSetup::OnBtnPresetFactory )
    EVT_BUTTON( widBtnPresetGrey, WizardQuickSetup::OnBtnPresetGrey )
    EVT_TOGGLEBUTTON( widBtnExposureAuto, WizardQuickSetup::OnBtnExposureAuto )
    EVT_TOGGLEBUTTON( widBtnGainAuto, WizardQuickSetup::OnBtnGainAuto )
    EVT_TOGGLEBUTTON( widBtnGamma, WizardQuickSetup::OnBtnGamma )
    EVT_TOGGLEBUTTON( widBtnWhiteBalanceAuto, WizardQuickSetup::OnBtnWhiteBalanceAuto )
    EVT_TOGGLEBUTTON( widBtnCCM, WizardQuickSetup::OnBtnCCM )
    EVT_TOGGLEBUTTON( widBtnFrameRateAuto, WizardQuickSetup::OnBtnFrameRateAuto )
    EVT_COMMAND_SCROLL_THUMBTRACK( widSLExposure, WizardQuickSetup::OnSLExposure )
    EVT_COMMAND_SCROLL_THUMBTRACK( widSLGain, WizardQuickSetup::OnSLGain )
    EVT_COMMAND_SCROLL_THUMBTRACK( widSLBlackLevel, WizardQuickSetup::OnSLBlackLevel )
    EVT_COMMAND_SCROLL_THUMBTRACK( widSLWhiteBalanceR, WizardQuickSetup::OnSLWhiteBalanceR )
    EVT_COMMAND_SCROLL_THUMBTRACK( widSLWhiteBalanceB, WizardQuickSetup::OnSLWhiteBalanceB )
    EVT_COMMAND_SCROLL_THUMBTRACK( widSLSaturation, WizardQuickSetup::OnSLSaturation )
    EVT_COMMAND_SCROLL_THUMBTRACK( widSLFrameRate, WizardQuickSetup::OnSLFrameRate )
    EVT_CLOSE( WizardQuickSetup::OnClose )
    EVT_SPINCTRL( widSCExposure, WizardQuickSetup::OnSCExposureChanged )
    EVT_SPINCTRL( widSCGain, WizardQuickSetup::OnSCGainChanged )
    EVT_SPINCTRL( widSCBlackLevel, WizardQuickSetup::OnSCBlackLevelChanged )
    EVT_SPINCTRL( widSCWhiteBalanceR, WizardQuickSetup::OnSCWhiteBalanceRChanged )
    EVT_SPINCTRL( widSCWhiteBalanceB, WizardQuickSetup::OnSCWhiteBalanceBChanged )
    EVT_SPINCTRL( widSCSaturation, WizardQuickSetup::OnSCSaturationChanged )
    EVT_SPINCTRL( widSCFrameRate, WizardQuickSetup::OnSCFrameRateChanged )
#ifdef BUILD_WITH_TEXT_EVENTS_FOR_SPINCTRL // BAT: Unfortunately on linux wxWidgets 2.6.x - ??? handling these messages will cause problems, while on Windows not doing so will not always update the GUI as desired :-(
    EVT_TEXT_ENTER( widSCExposure, WizardQuickSetup::OnSCExposureTextChanged )
    EVT_TEXT_ENTER( widSCGain, WizardQuickSetup::OnSCGainTextChanged )
    EVT_TEXT_ENTER( widSCBlackLevel, WizardQuickSetup::OnSCBlackLevelTextChanged )
    EVT_TEXT_ENTER( widSCWhiteBalanceR, WizardQuickSetup::OnSCWhiteBalanceRTextChanged )
    EVT_TEXT_ENTER( widSCWhiteBalanceB, WizardQuickSetup::OnSCWhiteBalanceBTextChanged )
    EVT_TEXT_ENTER( widSCSaturation, WizardQuickSetup::OnSCSaturationTextChanged )
    EVT_TEXT_ENTER( widSCFrameRate, WizardQuickSetup::OnSCFrameRateTextChanged )
#endif // #ifdef BUILD_WITH_TEXT_EVENTS_FOR_SPINCTRL
END_EVENT_TABLE()

const double        WizardQuickSetup::GAMMA_ = 2.;
const double        WizardQuickSetup::SLIDER_GRANULARITY_ = 100.;
const double        WizardQuickSetup::GAMMA_CORRECTION_VALUE_ = 1.8;

//-----------------------------------------------------------------------------
class SuspendAcquisitionScopeLock
//-----------------------------------------------------------------------------
{
    PropViewFrame* pParent_;
public:
    explicit SuspendAcquisitionScopeLock( PropViewFrame* pParent )
        : pParent_( pParent )
    {
        pParent_->EnsureAcquisitionState( false );
    }
    ~SuspendAcquisitionScopeLock()
    {
        pParent_->EnsureAcquisitionState( true );
    }
};

//-----------------------------------------------------------------------------
WizardQuickSetup::WizardQuickSetup( PropViewFrame* pParent, const wxString& title, bool boShowAtStartup )
    : OkAndCancelDlg( pParent, widMainFrame, title, wxDefaultPosition, wxDefaultSize, wxDEFAULT_DIALOG_STYLE | wxMINIMIZE_BOX ),
      pTopDownSizer_( 0 ), pBtnPresetColor_( 0 )/*, pBtnPresetColorHS_( 0 )*/, pBtnPresetFactory_( 0 ), pBtnPresetGrey_( 0 )/*, pBtnPresetGreyHS_( 0 )*/,
      pSLExposure_( 0 ), pSCExposure_( 0 ), pBtnExposureAuto_( 0 ), pCBShowDialogAtStartup_( 0 ), boGUILocked_( true ),
      pDev_( 0 ), pIP_( 0 ), propGridSettings_(), pParentPropViewFrame_( pParent ),  pID_( 0 )
//-----------------------------------------------------------------------------
{
    /*
        |-------------------------------------|
        | pTopDownSizer                       |
        |                spacer               |
        | |---------------------------------| |
        | | pPresetsSizer                   | |
        | |---------------------------------| |
        |                spacer               |
        | |---------------------------------| |
        | | pParametersSizer                | |
        | |---------------------------------| |
        |                spacer               |
        | |---------------------------------| |
        | | pSettingsSizer                  | |
        | |---------------------------------| |
        | |---------------------------------| |
        | | pButtonSizer                    | |
        | |---------------------------------| |
        |-------------------------------------|
        */
    wxPanel* pPanel = new wxPanel( this );

    // In the future, if GUI layout problems occur, a FlexGridSizer approach has to be considered!
    // 'Presets' controls
    //pBtnPresetColor_ = new wxButton( pPanel, widBtnPresetColor, wxT( "Color" ) );
    const wxBitmap colorPresetBitmap( wizard_color_preset_xpm );
    const wxBitmap colorPresetBitmapDisabled( wizard_color_preset_disabled_xpm );
    const wxBitmap grayPresetBitmap( wizard_gray_preset_xpm );
    const wxBitmap factoryPresetBitmap( wizard_factory_preset_xpm );
    pBtnPresetColor_ = new wxBitmapButton( pPanel, widBtnPresetColor, colorPresetBitmap, wxDefaultPosition, wxDefaultSize, wxBU_AUTODRAW, wxDefaultValidator, wxT( "High-Quality color" ) );
    pBtnPresetColor_->SetToolTip( wxT( "Will setup the device for optimal color fidelity.\nCurrent settings will be overwritten!" ) );
    pBtnPresetColor_->SetBitmapDisabled( colorPresetBitmapDisabled );
    pBtnPresetGrey_ = new wxBitmapButton( pPanel, widBtnPresetGrey, grayPresetBitmap, wxDefaultPosition, wxDefaultSize, wxBU_AUTODRAW, wxDefaultValidator, wxT( "High-Quality grayscale" ) );
    pBtnPresetGrey_->SetToolTip( wxT( "Will setup the device for optimal gray-scale image capture.\nCurrent settings will be overwritten!" ) );
    //pBtnPresetColorHS_ = new wxBitmapButton( pPanel, widBtnPresetColor, colorPresetBitmap, wxDefaultPosition, wxDefaultSize, wxBU_AUTODRAW, wxDefaultValidator, wxT( "High-Speed color" ) );
    //pBtnPresetColorHS_->SetToolTip( wxT( "COLOR HIGH SPEED:\nWill setup the device for optimal color settings for high-speed acquisition.\nCurrent settings will be overwritten!" ) );
    //pBtnPresetColorHS_->SetBitmapDisabled( colorPresetBitmapDisabled );
    //pBtnPresetGreyHS_ = new wxBitmapButton( pPanel, widBtnPresetGrey, grayPresetBitmap, wxDefaultPosition, wxDefaultSize, wxBU_AUTODRAW, wxDefaultValidator, wxT( "High-Speed grayscale" ) );
    //pBtnPresetGreyHS_->SetToolTip( wxT( "GRAYSCALE HIGH SPEED:\nWill setup the device for optimal gray-scalesettings for high-speed acquisition.\nCurrent settings will be overwritten!" ) );
    pBtnPresetFactory_ = new wxBitmapButton( pPanel, widBtnPresetFactory, factoryPresetBitmap, wxDefaultPosition, wxDefaultSize, wxBU_AUTODRAW, wxDefaultValidator, wxT( "Factory preset" ) );
    pBtnPresetFactory_->SetToolTip( wxT( "Will restore the factory default settings for this device!\n" ) );

    wxFlexGridSizer* pColorFlexGridSizer = new wxFlexGridSizer( 3, 2 );
    pColorFlexGridSizer->AddSpacer( 10 );
    pColorFlexGridSizer->Add( pBtnPresetColor_, wxSizerFlags().Center().Align( wxALIGN_CENTRE ) );
    pColorFlexGridSizer->AddSpacer( 10 );
    pColorFlexGridSizer->AddSpacer( 10 );
    pColorFlexGridSizer->Add( new wxStaticText( pPanel, wxID_ANY, wxT( " Color" ) ), wxSizerFlags().Center().Align( wxALIGN_CENTRE ) );
    pColorFlexGridSizer->AddSpacer( 10 );

    wxFlexGridSizer* pGrayscaleFlexGridSizer = new wxFlexGridSizer( 3, 2 );
    pGrayscaleFlexGridSizer->AddSpacer( 10 );
    pGrayscaleFlexGridSizer->Add( pBtnPresetGrey_, wxSizerFlags().Center().Align( wxALIGN_CENTRE ) );
    pGrayscaleFlexGridSizer->AddSpacer( 10 );
    pGrayscaleFlexGridSizer->AddSpacer( 10 );
    pGrayscaleFlexGridSizer->Add( new wxStaticText( pPanel, wxID_ANY, wxT( " Grayscale" ) ), wxSizerFlags().Center().Align( wxALIGN_CENTRE ) );
    pGrayscaleFlexGridSizer->AddSpacer( 10 );

    wxFlexGridSizer* pFactoryFlexGridSizer = new wxFlexGridSizer( 3, 2 );
    pFactoryFlexGridSizer->AddSpacer( 10 );
    pFactoryFlexGridSizer->Add( pBtnPresetFactory_, wxSizerFlags().Center().Align( wxALIGN_CENTRE ) );
    pFactoryFlexGridSizer->AddSpacer( 10 );
    pFactoryFlexGridSizer->AddSpacer( 10 );
    pFactoryFlexGridSizer->Add( new wxStaticText( pPanel, wxID_ANY, wxT( " Factory" ) ), wxSizerFlags().Center().Align( wxALIGN_CENTRE ) );
    pFactoryFlexGridSizer->AddSpacer( 10 );

    wxBoxSizer* pPresetsSizer = new wxStaticBoxSizer( wxHORIZONTAL, pPanel, wxT( "Presets: " ) );
    pPresetsSizer->Add( pColorFlexGridSizer );
    pPresetsSizer->AddSpacer( 10 );
    pPresetsSizer->Add( pGrayscaleFlexGridSizer );
    pPresetsSizer->AddStretchSpacer( 100 );
    pPresetsSizer->Add( pFactoryFlexGridSizer, wxSizerFlags().Right().Align( wxALIGN_RIGHT ) );

    // 'Parameters' controls
    pSLExposure_ = new wxSlider( pPanel, widSLExposure, 1000, -10000, 10000, wxDefaultPosition, wxSize( 250, -1 ), wxSL_HORIZONTAL );
    pSCExposure_ = new wxSpinCtrlDbl();
    pSCExposure_->Create( pPanel, widSCExposure, wxEmptyString, wxDefaultPosition, wxSize( 80, -1 ), wxSP_ARROW_KEYS, -10., 10., 1., 100, wxSPINCTRLDBL_AUTODIGITS, wxT( "%.0f" ) );
    pSCExposure_->SetMode( mDouble );
    pBtnExposureAuto_ = new wxToggleButton( pPanel, widBtnExposureAuto, wxT( "Auto" ) );

    wxBoxSizer* pExposureControlsSizer = new wxBoxSizer( wxHORIZONTAL );
    pExposureControlsSizer->Add( new wxStaticText( pPanel, wxID_ANY, wxT( " Exposure [us]:" ) ), wxSizerFlags( 3 ).Expand() );
    pExposureControlsSizer->Add( pSCExposure_, wxSizerFlags().Expand() );
    pExposureControlsSizer->Add( pSLExposure_, wxSizerFlags( 6 ).Expand() );
    pExposureControlsSizer->Add( pBtnExposureAuto_, wxSizerFlags().Expand() );

    pSLGain_ = new wxSlider( pPanel, widSLGain, 1000, -10000, 10000, wxDefaultPosition, wxSize( 250, -1 ), wxSL_HORIZONTAL );
    pSCGain_ = new wxSpinCtrlDbl();
    pSCGain_->Create( pPanel, widSCGain, wxEmptyString, wxDefaultPosition, wxSize( 80, -1 ), wxSP_ARROW_KEYS, -10., 10., 1., 0.001, wxSPINCTRLDBL_AUTODIGITS, wxT( "%.3f" ) );
    pSCGain_->SetMode( mDouble );
    pBtnGainAuto_ = new wxToggleButton( pPanel, widBtnGainAuto, wxT( "Auto" ) );

    wxBoxSizer* pGainControlsSizer = new wxBoxSizer( wxHORIZONTAL );
    pGainControlsSizer->Add( new wxStaticText( pPanel, wxID_ANY, wxT( " Gain [dB]:" ) ), wxSizerFlags( 3 ).Expand() );
    pGainControlsSizer->Add( pSCGain_, wxSizerFlags().Expand() );
    pGainControlsSizer->Add( pSLGain_, wxSizerFlags( 6 ).Expand() );
    pGainControlsSizer->Add( pBtnGainAuto_, wxSizerFlags().Expand() );

    pSLBlackLevel_ = new wxSlider( pPanel, widSLBlackLevel, 1000, -10000, 10000, wxDefaultPosition, wxSize( 250, -1 ), wxSL_HORIZONTAL );
    pSCBlackLevel_ = new wxSpinCtrlDbl();
    pSCBlackLevel_->Create( pPanel, widSCBlackLevel, wxEmptyString, wxDefaultPosition, wxSize( 80, -1 ), wxSP_ARROW_KEYS, -10., 10., 1., 0.001, wxSPINCTRLDBL_AUTODIGITS, wxT( "%.3f" ) );
    pSCBlackLevel_->SetMode( mDouble );

    wxBoxSizer* pBlackLevelControlsSizer = new wxBoxSizer( wxHORIZONTAL );
    pBlackLevelControlsSizer->Add( new wxStaticText( pPanel, wxID_ANY, wxT( " Black Level [%]:" ) ), wxSizerFlags( 3 ).Expand() );
    pBlackLevelControlsSizer->Add( pSCBlackLevel_, wxSizerFlags().Expand() );
    pBlackLevelControlsSizer->Add( pSLBlackLevel_, wxSizerFlags( 6 ).Expand() );
    pBlackLevelControlsSizer->Add( pBtnGainAuto_->GetSize().GetWidth(), 0, 0 );

    pSLSaturation_ = new wxSlider( pPanel, widSLSaturation, 1000, -10000, 10000, wxDefaultPosition, wxSize( 250, -1 ), wxSL_HORIZONTAL );
    pSCSaturation_ = new wxSpinCtrlDbl();
    pSCSaturation_->Create( pPanel, widSCSaturation, wxEmptyString, wxDefaultPosition, wxSize( 80, -1 ), wxSP_ARROW_KEYS, 1., 200., 100., 0.01, wxSPINCTRLDBL_AUTODIGITS, wxT( "%.2f" ) );
    pSCSaturation_->SetMode( mDouble );

    wxBoxSizer* pSaturationControlsSizer = new wxBoxSizer( wxHORIZONTAL );
    pSaturationControlsSizer->Add( new wxStaticText( pPanel, wxID_ANY, wxT( " Saturation [%]:" ) ), wxSizerFlags( 3 ).Expand() );
    pSaturationControlsSizer->Add( pSCSaturation_, wxSizerFlags().Expand() );
    pSaturationControlsSizer->Add( pSLSaturation_, wxSizerFlags( 6 ).Expand() );
    pSaturationControlsSizer->Add( pBtnGainAuto_->GetSize().GetWidth(), 0, 0 );

    const wxString ttWB( wxT( "This is in percent relative to the green gain" ) );
    pSLWhiteBalanceR_ = new wxSlider( pPanel, widSLWhiteBalanceR, 1000, -10000, 10000, wxDefaultPosition, wxSize( 250, -1 ), wxSL_HORIZONTAL );
    pSLWhiteBalanceR_->SetToolTip( ttWB );
    pSCWhiteBalanceR_ = new wxSpinCtrlDbl();
    pSCWhiteBalanceR_->Create( pPanel, widSCWhiteBalanceR, wxEmptyString, wxDefaultPosition, wxSize( 80, -1 ), wxSP_ARROW_KEYS, 1., 500., 100., 0.01, wxSPINCTRLDBL_AUTODIGITS, wxT( "%.2f" ) );
    pSCWhiteBalanceR_->SetMode( mDouble );
    pSCWhiteBalanceR_->SetToolTip( ttWB );
    pSLWhiteBalanceB_ = new wxSlider( pPanel, widSLWhiteBalanceB, 1000, -10000, 10000, wxDefaultPosition, wxSize( 250, -1 ), wxSL_HORIZONTAL );
    pSLWhiteBalanceB_->SetToolTip( ttWB );
    pSCWhiteBalanceB_ = new wxSpinCtrlDbl();
    pSCWhiteBalanceB_->Create( pPanel, widSCWhiteBalanceB, wxEmptyString, wxDefaultPosition, wxSize( 80, -1 ), wxSP_ARROW_KEYS, 1., 500., 100., 0.01, wxSPINCTRLDBL_AUTODIGITS, wxT( "%.2f" ) );
    pSCWhiteBalanceB_->SetMode( mDouble );
    pSCWhiteBalanceB_->SetToolTip( ttWB );
    pBtnWhiteBalanceAuto_ = new wxToggleButton( pPanel, widBtnWhiteBalanceAuto, wxT( "Auto" ) );

    wxBoxSizer* pWhiteBalanceDoubleSliderSizer = new wxBoxSizer( wxVERTICAL );
    pWhiteBalanceDoubleSliderSizer->AddSpacer( 5 );
    pWhiteBalanceDoubleSliderSizer->Add( pSLWhiteBalanceR_, wxSizerFlags( 6 ).Expand() );
    pWhiteBalanceDoubleSliderSizer->AddSpacer( 5 );
    pWhiteBalanceDoubleSliderSizer->Add( pSLWhiteBalanceB_, wxSizerFlags( 6 ).Expand() );

    wxBoxSizer* pWhiteBalanceDoubleSpinControlSizer = new wxBoxSizer( wxVERTICAL );
    pWhiteBalanceDoubleSpinControlSizer->Add( pSCWhiteBalanceR_, wxSizerFlags().Expand() );
    pWhiteBalanceDoubleSpinControlSizer->AddSpacer( 5 );
    pWhiteBalanceDoubleSpinControlSizer->Add( pSCWhiteBalanceB_, wxSizerFlags().Expand() );

    wxBoxSizer* pWhiteBalanceDoubleTextControlSizer = new wxBoxSizer( wxVERTICAL );
    wxStaticText* pSTWBR = new wxStaticText( pPanel, wxID_ANY, wxT( " White Balance R [%]:\n" ) );
    pSTWBR->SetToolTip( ttWB );
    wxStaticText* pSTWBB = new wxStaticText( pPanel, wxID_ANY, wxT( " White Balance B [%]:\n" ) );
    pSTWBB->SetToolTip( ttWB );
    pWhiteBalanceDoubleTextControlSizer->Add( pSTWBR, wxSizerFlags( 3 ) );
    pWhiteBalanceDoubleTextControlSizer->Add( pSTWBB, wxSizerFlags( 3 ) );

    wxBoxSizer* pWhiteBalanceControlsSizer = new wxBoxSizer( wxHORIZONTAL );
    pWhiteBalanceControlsSizer->Add( pWhiteBalanceDoubleTextControlSizer, wxSizerFlags( 2 ).Expand() );
    pWhiteBalanceControlsSizer->Add( pWhiteBalanceDoubleSpinControlSizer );
    pWhiteBalanceControlsSizer->Add( pWhiteBalanceDoubleSliderSizer );
    pWhiteBalanceControlsSizer->Add( pBtnWhiteBalanceAuto_, wxSizerFlags().Top().Expand() );

    pBtnCCM_ = new wxToggleButton( pPanel, widBtnCCM, wxT( "Color+" ) );
    pBtnCCM_->SetToolTip( wxT( "Will apply sensor-specific CCM & sRGB transformations. Only use with a monitor configured to display sRGB!" ) );
    pBtnGamma_ = new wxToggleButton( pPanel, widBtnGamma, wxT( "Gamma" ) );

    wxBoxSizer* pButtonsSizer = new wxBoxSizer( wxHORIZONTAL );
    pButtonsSizer->Add( pBtnGamma_, wxSizerFlags().Expand() );
    pButtonsSizer->AddSpacer( 10 );
    pButtonsSizer->Add( pBtnCCM_, wxSizerFlags().Expand() );

    pSLFrameRate_ = new wxSlider( pPanel, widSLFrameRate, 1000, -10000, 10000, wxDefaultPosition, wxSize( 250, -1 ), wxSL_HORIZONTAL );
    pSCFrameRate_ = new wxSpinCtrlDbl();
    pSCFrameRate_->Create( pPanel, widSCFrameRate, wxEmptyString, wxDefaultPosition, wxSize( 80, -1 ), wxSP_ARROW_KEYS, -10., 10., 1., 0.001, wxSPINCTRLDBL_AUTODIGITS, wxT( "%.3f" ) );
    pSCFrameRate_->SetMode( mDouble );
    pBtnFrameRateAuto_ = new wxToggleButton( pPanel, widBtnFrameRateAuto, wxT( "Auto" ) );

    wxBoxSizer* pFrameRateControlsSizer = new wxBoxSizer( wxHORIZONTAL );
    pFrameRateControlStaticText_ = new wxStaticText( pPanel, wxID_ANY, wxT( " Frame Rate [Hz]:" ) );
    pFrameRateControlsSizer->Add( pFrameRateControlStaticText_, wxSizerFlags( 3 ).Expand() );
    pFrameRateControlsSizer->Add( pSCFrameRate_, wxSizerFlags().Expand() );
    pFrameRateControlsSizer->Add( pSLFrameRate_, wxSizerFlags( 6 ).Expand() );
    pFrameRateControlsSizer->Add( pBtnFrameRateAuto_, wxSizerFlags().Expand() );

    wxBoxSizer* pParametersSizer = new wxStaticBoxSizer( wxVERTICAL, pPanel, wxT( "Parameters: " ) );
    pParametersSizer->Add( pExposureControlsSizer, wxSizerFlags().Expand() );
    pParametersSizer->AddSpacer( 12 );
    pParametersSizer->Add( pGainControlsSizer, wxSizerFlags().Expand() );
    pParametersSizer->AddSpacer( 12 );
    pParametersSizer->Add( pBlackLevelControlsSizer, wxSizerFlags().Expand() );
    pParametersSizer->AddSpacer( 12 );
    pParametersSizer->Add( pSaturationControlsSizer, wxSizerFlags().Expand() );
    pParametersSizer->AddSpacer( 12 );
    pParametersSizer->Add( pWhiteBalanceControlsSizer, wxSizerFlags().Expand() );
    pParametersSizer->AddSpacer( 5 );
    pParametersSizer->Add( pButtonsSizer, wxSizerFlags().Expand().Right().Align( wxALIGN_RIGHT ) );
    pParametersSizer->AddSpacer( 12 );
    pParametersSizer->Add( pFrameRateControlsSizer, wxSizerFlags().Expand() );

    // 'Settings' controls
    pCBShowDialogAtStartup_ = new wxCheckBox( pPanel, widCBShowDialogAtStartup, wxT( "Show This Dialog When A Device Is Opened" ) );
    pCBShowDialogAtStartup_->SetValue( boShowAtStartup );

    wxBoxSizer* pSettingsSizer = new wxStaticBoxSizer( wxVERTICAL, pPanel, wxT( "Settings: " ) );
    pSettingsSizer->AddSpacer( 5 );
    pSettingsSizer->Add( pCBShowDialogAtStartup_, wxSizerFlags().Expand() );
    pSettingsSizer->AddSpacer( 5 );

    // putting it all together
    pTopDownSizer_ = new wxBoxSizer( wxVERTICAL );
    pTopDownSizer_->AddSpacer( 12 );
    pTopDownSizer_->Add( pPresetsSizer, wxSizerFlags().Expand() );
    pTopDownSizer_->AddSpacer( 12 );
    pTopDownSizer_->Add( pParametersSizer, wxSizerFlags().Expand() );
    pTopDownSizer_->AddSpacer( 12 );
    pTopDownSizer_->Add( pSettingsSizer, wxSizerFlags().Expand() );
    pTopDownSizer_->AddSpacer( 12 );

    AddButtons( pPanel, pTopDownSizer_, false );

    wxBoxSizer* pOuterSizer = new wxBoxSizer( wxHORIZONTAL );
    pOuterSizer->AddSpacer( 5 );
    pOuterSizer->Add( pTopDownSizer_, wxSizerFlags().Expand() );
    pOuterSizer->AddSpacer( 5 );

    FinalizeDlgCreation( pPanel, pOuterSizer );
    boGUILocked_ = false;
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::AnalyzeDeviceAndGatherInformation( SupportedWizardFeatures& supportedFeatures )
//-----------------------------------------------------------------------------
{
    try
    {
        pBtnPresetFactory_->Enable( HasFactoryDefault() );
        const bool hasColorFormat = HasColorFormat();
        supportedFeatures.boColorOptionsSupport = hasColorFormat;
        currentSettings_[currentDeviceSerial_].boColorEnabled = hasColorFormat;
        supportedFeatures.boAutoExposureSupport = HasAEC();
        supportedFeatures.boAutoGainSupport = HasAGC();
        supportedFeatures.boAutoWhiteBalanceSupport = HasAWB();
        supportedFeatures.boAutoFrameRateSupport = HasAutoFrameRate();
        supportedFeatures.boRegulateFrameRateSupport = HasFrameRateEnable();
    }
    catch( const ImpactAcquireException& e )
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to analyse Device (Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::CleanUp( void )
//-----------------------------------------------------------------------------
{
    pDev_ = 0;
    DeleteInterfaceLayoutSpecificControls();
    DeleteElement( pID_ );
    DeleteElement( pIP_ );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::CloseDlg( void )
//-----------------------------------------------------------------------------
{
    SuspendAcquisitionScopeLock suspendAcquisitionLock( pParentPropViewFrame_ );
    DeviceSettings devSettings = propGridSettings_[currentDeviceSerial_];
    SupportedWizardFeatures features = featuresSupported_[currentDeviceSerial_];
    try
    {
        if( pDev_->isOpen() &&
            ( pDev_->state.read() == dsPresent ) )
        {
            WriteExposureFeatures( devSettings, features );
            WriteUnifiedGainData( devSettings.unifiedGain );
            WriteGainFeatures( devSettings, features );
            WriteUnifiedBlackLevelData( devSettings.unifiedBlackLevel );
            pIP_->LUTEnable.write( devSettings.boGammaEnabled ? bTrue : bFalse );

            if( features.boColorOptionsSupport )
            {
                WriteWhiteBalanceFeatures( devSettings, features );
                SetFrameRateEnable( true );
                WriteSaturationData( devSettings.saturation );
                pIP_->colorTwistInputCorrectionMatrixEnable.write( devSettings.boCCMEnabled ? bTrue : bFalse );
                pIP_->colorTwistOutputCorrectionMatrixEnable.write( devSettings.boCCMEnabled ? bTrue : bFalse );
            }

            if( features.boAutoFrameRateSupport )
            {
                if( devSettings.boAutoFrameRateEnabled )
                {
                    SetFrameRateEnable( false );
                }
                else
                {
                    SetFrameRateEnable( true );
                    SetFrameRate( devSettings.frameRate );
                }
            }

            SetPixelFormat( devSettings.imageFormatControlPixelFormat );
            pID_->pixelFormat.writeS( devSettings.imageDestinationPixelFormat );
            SelectLUTDependingOnPixelFormat();
        }
    }
    catch( const ImpactAcquireException& e )
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Error when closing dialog: %s(%s)" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
    }

    //The user cancelled, therefore, the original settings before calling the wizard, have to be set as
    //current ones, otherwise there may be inconsistencies when calling the wizard again.
    currentSettings_[currentDeviceSerial_] = devSettings;
    Hide();
    pParentPropViewFrame_->RestoreGUIStateAfterQuickSetupWizard();
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::ApplyExposure( void )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        try
        {
            SetExposureTime( pSCExposure_->GetValue() );
        }
        catch( const ImpactAcquireException& e )
        {
            WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to apply exposure time(Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
        }
    }
    //The exposure is a special case. It influences the FrameRate properties, thus the FrameRate controls have to be redrawn
    if( !currentSettings_[currentDeviceSerial_].boAutoFrameRateEnabled )
    {
        SetupFrameRateControls();
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::ApplyGain( void )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        try
        {
            WriteUnifiedGainData( pSCGain_->GetValue() );
        }
        catch( const ImpactAcquireException& e )
        {
            WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to apply gain(Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
        }
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::ApplyBlackLevel( void )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        try
        {
            WriteUnifiedBlackLevelData( pSCBlackLevel_->GetValue() );
        }
        catch( const ImpactAcquireException& e )
        {
            WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to apply gain(Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
        }
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::ApplyWhiteBalance( TWhiteBalanceChannel channel )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        if( featuresSupported_[currentDeviceSerial_].boColorOptionsSupport )
        {
            try
            {
                SetWhiteBalance( channel, ( ( channel == wbcRed ) ? pSCWhiteBalanceR_->GetValue() : pSCWhiteBalanceB_->GetValue() ) / 100.0 );
            }
            catch( const ImpactAcquireException& e )
            {
                WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to apply whitebalance (Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
            }
        }
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::ApplySaturation( void )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        if( featuresSupported_[currentDeviceSerial_].boColorOptionsSupport )
        {
            try
            {
                WriteSaturationData( pSCSaturation_->GetValue() );
            }
            catch( const ImpactAcquireException& e )
            {
                WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to apply saturation(Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
            }
        }
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::ApplyFrameRate( void )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        try
        {
            SetFrameRateEnable( true );
            SetFrameRate( pSCFrameRate_->GetValue() );
        }
        catch( const ImpactAcquireException& e )
        {
            WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to apply framerate limit(Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
        }
        // Since framerate controls are special due to their dynamic nature ( they depend on the exposure values )
        // many functions rely on the framerate member of the Device Settings structure. Thus we have to keep it
        // up to date with every change.
        currentSettings_[currentDeviceSerial_].frameRate = pSCFrameRate_->GetValue();
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::DoSetupExposureControls( double exposureMin, double exposureMax, double exposure, bool boHasStepWidth, double increment )
//-----------------------------------------------------------------------------
{
    VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
    boGUILocked_ = true;
    pSLExposure_->SetRange( static_cast< int >( exposureMin ), static_cast< int >( exposureMax ) );
    pSCExposure_->SetRange( exposureMin, exposureMax );
    if( boHasStepWidth )
    {
        pSCExposure_->SetIncrement( increment );
    }
    pSCExposure_->SetValue( exposure );
    pSLExposure_->SetValue( ExposureToSliderValue( exposure ) );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::DoSetupGainControls( double gainUnifiedRangeMin, double gainUnifiedRangeMax, double gain )
//-----------------------------------------------------------------------------
{
    VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
    boGUILocked_ = true;
    pSLGain_->SetRange( static_cast< int >( gainUnifiedRangeMin * SLIDER_GRANULARITY_ ), static_cast< int >( gainUnifiedRangeMax * SLIDER_GRANULARITY_ ) );
    pSCGain_->SetRange( gainUnifiedRangeMin, gainUnifiedRangeMax );
    pSCGain_->SetIncrement( 1 / SLIDER_GRANULARITY_ );
    pSCGain_->SetValue( gain );
    pSLGain_->SetValue( static_cast< int >( gain * SLIDER_GRANULARITY_ ) );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::DoSetupBlackLevelControls( double blackLevelUnifiedRangeMin, double blackLevelUnifiedRangeMax, double blackLevel )
//-----------------------------------------------------------------------------
{
    VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
    boGUILocked_ = true;
    pSLBlackLevel_->SetRange( static_cast< int >( blackLevelUnifiedRangeMin * SLIDER_GRANULARITY_ ), static_cast< int >( blackLevelUnifiedRangeMax * SLIDER_GRANULARITY_ ) );
    pSCBlackLevel_->SetRange( blackLevelUnifiedRangeMin, blackLevelUnifiedRangeMax );
    pSCBlackLevel_->SetIncrement( 1 / SLIDER_GRANULARITY_ );
    pSCBlackLevel_->SetValue( blackLevel );
    pSLBlackLevel_->SetValue( static_cast< int >( blackLevel * SLIDER_GRANULARITY_ ) );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::DoSetupWhiteBalanceControls( double whiteBalanceRMin, double whiteBalanceRMax, double whiteBalanceR, double whiteBalanceBMin, double whiteBalanceBMax, double whiteBalanceB )
//-----------------------------------------------------------------------------
{
    VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
    boGUILocked_ = true;
    pSLWhiteBalanceR_->SetRange( static_cast< int >( whiteBalanceRMin * SLIDER_GRANULARITY_ ), static_cast< int >( whiteBalanceRMax * SLIDER_GRANULARITY_ ) );
    pSCWhiteBalanceR_->SetRange( whiteBalanceRMin, whiteBalanceRMax );
    pSCWhiteBalanceR_->SetIncrement( 1 / SLIDER_GRANULARITY_ );
    pSCWhiteBalanceR_->SetValue( whiteBalanceR );
    pSLWhiteBalanceR_->SetValue( static_cast< int >( whiteBalanceR * SLIDER_GRANULARITY_ ) );

    pSLWhiteBalanceB_->SetRange( static_cast< int >( whiteBalanceBMin * SLIDER_GRANULARITY_ ), static_cast< int >( whiteBalanceBMax * SLIDER_GRANULARITY_ ) );
    pSCWhiteBalanceB_->SetRange( whiteBalanceBMin, whiteBalanceBMax );
    pSCWhiteBalanceB_->SetIncrement( 1 / SLIDER_GRANULARITY_ );
    pSCWhiteBalanceB_->SetValue( whiteBalanceB );
    pSLWhiteBalanceB_->SetValue( static_cast< int >( whiteBalanceB * SLIDER_GRANULARITY_ ) );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::DoSetupFrameRateControls( double frameRateRangeMin, double frameRateRangeMax, double frameRate )
//-----------------------------------------------------------------------------
{
    VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
    boGUILocked_ = true;
    pSLFrameRate_->SetRange( static_cast< int >( frameRateRangeMin * SLIDER_GRANULARITY_ ), static_cast< int >( frameRateRangeMax * SLIDER_GRANULARITY_ ) );
    pSCFrameRate_->SetRange( frameRateRangeMin, frameRateRangeMax );
    pSCFrameRate_->SetIncrement( 1 / SLIDER_GRANULARITY_ );
    pSCFrameRate_->SetValue( frameRate );
    pSLFrameRate_->SetValue( static_cast< int >( frameRate * SLIDER_GRANULARITY_ ) );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::OnClose( wxCloseEvent& e )
//-----------------------------------------------------------------------------
{
    CloseDlg();
    if( e.CanVeto() )
    {
        e.Veto();
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::OnBtnPresetColor( wxCommandEvent& )
//-----------------------------------------------------------------------------
{
    SuspendAcquisitionScopeLock suspendAcquisitionLock( pParentPropViewFrame_ );
    SetupDevice();
    PresetColorHQ();
    SetupControls();
    WriteQuickSetupWizardLogMessage( wxString::Format( wxT( "Using quality-optimized color presets for device %s(%s)" ), ConvertedString( currentProductString_ ).c_str(), ConvertedString( currentDeviceSerial_ ).c_str() ) );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::OnBtnPresetCustom( wxCommandEvent& )
//-----------------------------------------------------------------------------
{
    wxMessageBox( wxT( "Nothing implemented for the 'Custom' preset so far" ), wxT( "Under Construction!" ), wxOK | wxICON_INFORMATION, this );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::OnBtnPresetFactory( wxCommandEvent& )
//-----------------------------------------------------------------------------
{
    const bool boShallDoFactoryReset = ShowFactoryResetPopup();
    if( boShallDoFactoryReset == false )
    {
        return;
    }
    SuspendAcquisitionScopeLock suspendAcquisitionLock( pParentPropViewFrame_ );

    try
    {
        RestoreFactoryDefault();
        SetAcquisitionFrameRateLimitMode();
    }
    catch( const ImpactAcquireException& e )
    {
        wxMessageBox( wxString::Format( wxT( "Failed to restore factory settings(Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ), wxT( "Error" ), wxOK | wxICON_INFORMATION, this );
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to restore factory settings(Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
    }

    pID_->restoreDefault();
    pIP_->restoreDefault();
    QueryInitialDeviceSettings( currentSettings_[currentDeviceSerial_] );
    if( featuresSupported_[currentDeviceSerial_].boColorOptionsSupport )
    {
        currentSettings_[currentDeviceSerial_].boColorEnabled = true;
    }
    SetupDriverSettings();
    SetupControls();

    //When making a factory reset the currentSettings and PropgridSettings for this device should
    //be overwritten with the factory settings, otherwise strange things may happen when pressing
    //the cancel button.
    SaveWizardConfiguration();
    propGridSettings_[currentDeviceSerial_] = currentSettings_[currentDeviceSerial_];
    SelectLUTDependingOnPixelFormat();
    WriteQuickSetupWizardLogMessage( wxString::Format( wxT( "Restored factory presets for device %s(%s)" ), ConvertedString( currentProductString_ ).c_str(), ConvertedString( currentDeviceSerial_ ).c_str() ) );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::OnBtnPresetGrey( wxCommandEvent& )
//-----------------------------------------------------------------------------
{
    SuspendAcquisitionScopeLock suspendAcquisitionLock( pParentPropViewFrame_ );
    SetupDevice();
    PresetGreyHQ();
    SetupControls();
    WriteQuickSetupWizardLogMessage( wxString::Format( wxT( "Using quality-optimized grayscale presets for device %s(%s)" ), ConvertedString( currentProductString_ ).c_str(), ConvertedString( currentDeviceSerial_ ).c_str() ) );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::ConfigureExposureAuto( bool boActive )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        if( featuresSupported_[currentDeviceSerial_].boAutoExposureSupport )
        {
            try
            {
                SetAutoExposure( boActive );
                if( boActive )
                {
                    currentSettings_[currentDeviceSerial_].exposureTime = pSCExposure_->GetValue();
                }
                else
                {
                    UpdateExposureControlsFromCamera();
                }
                currentSettings_[currentDeviceSerial_].boAutoExposureEnabled = boActive;
            }
            catch( const ImpactAcquireException& e )
            {
                WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to modify driver properties(Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
            }
        }
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::ConfigureGainAuto( bool boActive )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        if( featuresSupported_[currentDeviceSerial_].boAutoGainSupport )
        {
            try
            {
                SetAutoGain( boActive );
                if( boActive )
                {
                    currentSettings_[currentDeviceSerial_].unifiedGain = pSCGain_->GetValue();
                    // DigitalGain has to be set to 0, or else it's value will stack with the auto-gain values!
                    WriteUnifiedGainData( 0 );
                }
                else
                {
                    UpdateGainControlsFromCamera();
                }
                currentSettings_[currentDeviceSerial_].boAutoGainEnabled = boActive;
            }
            catch( const ImpactAcquireException& e )
            {
                WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to modify driver properties(Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
            }
        }
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::ConfigureGamma( bool boActive )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        try
        {
            pIP_->LUTEnable.write( boActive ? bTrue : bFalse );
            currentSettings_[currentDeviceSerial_].boGammaEnabled = boActive;
        }
        catch( const ImpactAcquireException& e )
        {
            WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to modify driver properties(Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
        }
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::ConfigureWhiteBalanceAuto( bool boActive )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        if( featuresSupported_[currentDeviceSerial_].boAutoWhiteBalanceSupport )
        {
            try
            {
                SetAutoWhiteBalance( boActive );
                if( boActive )
                {
                    currentSettings_[currentDeviceSerial_].whiteBalanceRed = pSCWhiteBalanceR_->GetValue();
                    currentSettings_[currentDeviceSerial_].whiteBalanceBlue = pSCWhiteBalanceB_->GetValue();
                }
                else
                {
                    UpdateWhiteBalanceControlsFromCamera();
                }
                currentSettings_[currentDeviceSerial_].boAutoWhiteBalanceEnabled = boActive;
            }
            catch( const ImpactAcquireException& e )
            {
                WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to modify driver properties(Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
            }
        }
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::ConfigureCCM( bool boActive )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        try
        {
            pIP_->colorTwistInputCorrectionMatrixEnable.write( boActive ? bTrue : bFalse );
            pIP_->colorTwistOutputCorrectionMatrixEnable.write( boActive ? bTrue : bFalse );
            currentSettings_[currentDeviceSerial_].boCCMEnabled = boActive;
        }
        catch( const ImpactAcquireException& e )
        {
            WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to modify driver properties(Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
        }
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::ConfigureFrameRateAuto( bool boActive )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        if( featuresSupported_[currentDeviceSerial_].boAutoFrameRateSupport )
        {
            try
            {
                DoConfigureFrameRateAuto( boActive, pSCFrameRate_->GetValue() );
                currentSettings_[currentDeviceSerial_].boAutoFrameRateEnabled = boActive;
            }
            catch( const ImpactAcquireException& e )
            {
                WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to modify driver properties(Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
            }
        }
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::HandleExposureSpinControlChanges( void )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        {
            VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
            boGUILocked_ = true;
            pSLExposure_->SetValue( ExposureToSliderValue( pSCExposure_->GetValue() ) );
        }
        ApplyExposure();
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::HandleGainSpinControlChanges( void )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        {
            VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
            boGUILocked_ = true;
            pSLGain_->SetValue( pSCGain_->GetValue()*SLIDER_GRANULARITY_ );
        }
        ApplyGain();
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::HandleBlackLevelSpinControlChanges( void )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        {
            VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
            boGUILocked_ = true;
            pSLBlackLevel_->SetValue( pSCBlackLevel_->GetValue()*SLIDER_GRANULARITY_ );
        }
        ApplyBlackLevel();
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::HandleWhiteBalanceRSpinControlChanges( void )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        {
            VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
            boGUILocked_ = true;
            pSLWhiteBalanceR_->SetValue( pSCWhiteBalanceR_->GetValue()*SLIDER_GRANULARITY_ );
        }
        ApplyWhiteBalance( wbcRed );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::HandleWhiteBalanceBSpinControlChanges( void )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        {
            VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
            boGUILocked_ = true;
            pSLWhiteBalanceB_->SetValue( pSCWhiteBalanceB_->GetValue()*SLIDER_GRANULARITY_ );
        }
        ApplyWhiteBalance( wbcBlue );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::HandleSaturationSpinControlChanges( void )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        {
            VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
            boGUILocked_ = true;
            pSLSaturation_->SetValue( pSCSaturation_->GetValue()*SLIDER_GRANULARITY_ );
        }
        ApplySaturation();
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::HandleFrameRateSpinControlChanges( void )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        {
            VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
            boGUILocked_ = true;
            pSLFrameRate_->SetValue( pSCFrameRate_->GetValue()*SLIDER_GRANULARITY_ );
        }
        ApplyFrameRate();
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::QueryInitialDeviceSettings( DeviceSettings&  devSettings )
//-----------------------------------------------------------------------------
{
    devSettings.boAutoExposureEnabled = false;
    devSettings.boAutoGainEnabled = false;
    devSettings.boGammaEnabled = false;
    devSettings.boAutoFrameRateEnabled = false;
    devSettings.boColorEnabled = false;
    devSettings.boCCMEnabled = false;

    QueryInterfaceLayoutSpecificSettings( devSettings );
    if( HasUnifiedGain() )
    {
        devSettings.unifiedGain = ReadUnifiedGainData();
    }
    devSettings.boGammaEnabled = pIP_->LUTEnable.read() == bTrue;
    devSettings.saturation = ReadSaturationData();
    devSettings.boCCMEnabled = ( pIP_->colorTwistInputCorrectionMatrixEnable.readS() == string( "On" ) &&
                                 pIP_->colorTwistOutputCorrectionMatrixEnable.readS() == string( "On" ) );

    TryToReadFrameRate( devSettings.frameRate );

    if( HasFrameRateEnable() )
    {
        devSettings.boAutoFrameRateEnabled = GetFrameRateEnable();
    }

    devSettings.analogGainMin = analogGainMin_;
    devSettings.analogGainMax = analogGainMax_;
    devSettings.digitalGainMin = digitalGainMin_;
    devSettings.digitalGainMax = digitalGainMax_;
    devSettings.analogBlackLevelMin = analogBlackLevelMin_;
    devSettings.analogBlackLevelMax = analogBlackLevelMax_;
    devSettings.digitalBlackLevelMin = digitalBlackLevelMin_;
    devSettings.digitalBlackLevelMax = digitalBlackLevelMax_;
    devSettings.imageFormatControlPixelFormat = GetPixelFormat();
    if( pID_->pixelFormat.isValid() )
    {
        devSettings.imageDestinationPixelFormat = pID_->pixelFormat.readS();
    }
}

//-----------------------------------------------------------------------------
double WizardQuickSetup::ExposureFromSliderValue( void ) const
//-----------------------------------------------------------------------------
{
    const int value = pSLExposure_->GetValue();
    const int valueMax = pSLExposure_->GetMax();
    return pow( static_cast< double >( value ) / static_cast< double >( valueMax ), GAMMA_ ) * static_cast< double >( valueMax );
}

//-----------------------------------------------------------------------------
int WizardQuickSetup::ExposureToSliderValue( const double exposure ) const
//-----------------------------------------------------------------------------
{
    const double exposureMax = pSCExposure_->GetMax();
    return static_cast< int >( pow( exposure / exposureMax, 1. / GAMMA_ ) * exposureMax );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::OnSLExposure( wxScrollEvent& )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        {
            VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
            boGUILocked_ = true;
            pSCExposure_->SetValue( static_cast< int >( ExposureFromSliderValue() ) );
        }
        ApplyExposure();
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::OnSLGain( wxScrollEvent& )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        {
            VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
            boGUILocked_ = true;
            pSCGain_->SetValue( static_cast< double >( pSLGain_->GetValue() ) / SLIDER_GRANULARITY_ );
        }
        ApplyGain();
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::OnSLBlackLevel( wxScrollEvent& )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        {
            VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
            boGUILocked_ = true;
            pSCBlackLevel_->SetValue( static_cast< double >( pSLBlackLevel_->GetValue() ) / SLIDER_GRANULARITY_ );
        }
        ApplyBlackLevel();
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::OnSLWhiteBalanceR( wxScrollEvent& )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        {
            VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
            boGUILocked_ = true;
            pSCWhiteBalanceR_->SetValue( static_cast< double >( pSLWhiteBalanceR_->GetValue() ) / SLIDER_GRANULARITY_ );
        }
        ApplyWhiteBalance( wbcRed );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::OnSLWhiteBalanceB( wxScrollEvent& )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        {
            VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
            boGUILocked_ = true;
            pSCWhiteBalanceB_->SetValue( static_cast< double >( pSLWhiteBalanceB_->GetValue() ) / SLIDER_GRANULARITY_ );
        }
        ApplyWhiteBalance( wbcBlue );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::OnSLSaturation( wxScrollEvent& )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        {
            VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
            boGUILocked_ = true;
            pSCSaturation_->SetValue( static_cast< double >( pSLSaturation_->GetValue() ) / SLIDER_GRANULARITY_ );
        }
        ApplySaturation();
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::OnSLFrameRate( wxScrollEvent& )
//-----------------------------------------------------------------------------
{
    if( boGUILocked_ == false )
    {
        {
            VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
            boGUILocked_ = true;
            pSCFrameRate_->SetValue( static_cast< double >( pSLFrameRate_->GetValue() ) / SLIDER_GRANULARITY_ );
        }
        ApplyFrameRate();
    }
}

//-----------------------------------------------------------------------------
double WizardQuickSetup::ReadSaturationData( void )
//-----------------------------------------------------------------------------
{
    double currentSaturation = 0.;
    try
    {
        currentSaturation = ( ( pIP_->colorTwistRow0.read( 0 ) - 0.299 ) / 0.701 ) * 100.0;
    }
    catch( const ImpactAcquireException& e )
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to read saturation value(Error: %s(%s))!" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
    }
    return currentSaturation;
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::WriteSaturationData( double saturation )
//-----------------------------------------------------------------------------
{
    try
    {
        if( pIP_->colorTwistEnable.readS() != string( "On" ) )
        {
            pIP_->colorTwistEnable.writeS( "On" );
        }
        pIP_->setSaturation( saturation / 100.0 );
    }
    catch( const ImpactAcquireException& e )
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to write saturation value(Error: %s(%s))!" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
    }
}

//-----------------------------------------------------------------------------
double WizardQuickSetup::ReadUnifiedGainData( void )
//-----------------------------------------------------------------------------
{
    double currentGain = 0.0;
    try
    {
        currentGain = DoReadUnifiedGain();
    }
    catch( const ImpactAcquireException& e )
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to read combined analog and digital gain values:\n%s(%s)" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
    }
    return currentGain;
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::WriteUnifiedGainData( double unifiedGain )
//-----------------------------------------------------------------------------
{
    try
    {
        DoWriteUnifiedGain( unifiedGain );
    }
    catch( const ImpactAcquireException& e )
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to write combined analog and digital gain values:\n%s(%s)" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
    }
}

//-----------------------------------------------------------------------------
double WizardQuickSetup::ReadUnifiedBlackLevelData()
//-----------------------------------------------------------------------------
{
    double currentBlackLevel = 0.0;
    try
    {
        currentBlackLevel = DoReadUnifiedBlackLevel();
    }
    catch( const ImpactAcquireException& e )
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to read combined analog and digital blackLevel values:\n%s(%s)" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
    }
    return currentBlackLevel;
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::WriteUnifiedBlackLevelData( double unifiedBlackLevel )
//-----------------------------------------------------------------------------
{
    try
    {
        DoWriteUnifiedBlackLevelData( unifiedBlackLevel );
    }
    catch( const ImpactAcquireException& e )
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to write combined analog and digital blackLevel values:\n%s(%s)" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::PresetColorHQ( void )
//-----------------------------------------------------------------------------
{
    currentSettings_[currentDeviceSerial_].boColorEnabled = true;
    WriteSaturationData( 100. );
    ConfigureCCM( true );
    ConfigureGamma( true );
    SelectColorPixelFormat();
    SelectLUTDependingOnPixelFormat();
    pID_->pixelFormat.write( idpfRGBx888Packed );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::PresetGreyHQ( void )
//-----------------------------------------------------------------------------
{
    currentSettings_[currentDeviceSerial_].boColorEnabled = false;
    ConfigureCCM( false );
    ConfigureGamma( false );
    SelectGreyscalePixelFormat();
    SelectLUTDependingOnPixelFormat();
    pID_->pixelFormat.write( idpfMono8 );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::ReferToNewDevice( Device* pDev )
//-----------------------------------------------------------------------------
{
    currentDeviceSerial_ = pDev->serial.readS();
    currentProductString_ = pDev->product.readS();
    bool boFirstTimeDeviceStartsWizard = ( currentSettings_.find( currentDeviceSerial_ ) == currentSettings_.end() );

    CleanUp();
    pDev_ = pDev;
    CreateInterfaceLayoutSpecificControls( pDev );
    pID_ = new ImageDestination( pDev );
    pIP_ = new ImageProcessing( pDev );

    SetupUnifiedData( boFirstTimeDeviceStartsWizard );
    //Save PropGridState in case Cancel is pressed.
    DeviceSettings devSettings;
    QueryInitialDeviceSettings( devSettings );
    propGridSettings_[currentDeviceSerial_] = devSettings;

    if( boFirstTimeDeviceStartsWizard )
    {
        currentSettings_[currentDeviceSerial_] = devSettings;
        SupportedWizardFeatures supportedFeatures;
        AnalyzeDeviceAndGatherInformation( supportedFeatures );
        featuresSupported_[currentDeviceSerial_] = supportedFeatures;
    }
    SetupDriverSettings();
    if( boFirstTimeDeviceStartsWizard )
    {
        SuspendAcquisitionScopeLock suspendAcquisitionLock( pParentPropViewFrame_ );
        SetupDevice();
        if( featuresSupported_[currentDeviceSerial_].boColorOptionsSupport )
        {
            PresetColorHQ();
        }
        else
        {
            PresetGreyHQ();
        }
    }
    else
    {
        RestoreWizardConfiguration();
        SelectLUTDependingOnPixelFormat();
    }
    SetupControls();

    SetTitle( wxString::Format( wxT( "Quick Setup [%s - %s] (%s)" ), ConvertedString( currentProductString_ ).c_str(), ConvertedString( pDev->serial.readS() ).c_str(), ConvertedString( pDev->interfaceLayout.readS() ).c_str() ) );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::RefreshControls( void )
//-----------------------------------------------------------------------------
{
    SupportedWizardFeatures features = featuresSupported_[currentDeviceSerial_];
    DeviceSettings settings = currentSettings_[currentDeviceSerial_];

    const bool boAutoExposureSupported = features.boAutoExposureSupport;
    const bool boExposureAutoIsActive = settings.boAutoExposureEnabled;
    pBtnExposureAuto_->SetValue( boExposureAutoIsActive );
    pBtnExposureAuto_->Enable( boAutoExposureSupported );
    pSLExposure_->Enable( !boAutoExposureSupported || !boExposureAutoIsActive );
    pSCExposure_->Enable( !boAutoExposureSupported || !boExposureAutoIsActive );

    const bool boAutoGainSupported = features.boAutoGainSupport;
    const bool boGainAutoIsActive = settings.boAutoGainEnabled;
    pBtnGainAuto_->SetValue( boGainAutoIsActive );
    pBtnGainAuto_->Enable( boAutoGainSupported );
    pSLGain_->Enable( !boAutoGainSupported || !boGainAutoIsActive );
    pSCGain_->Enable( !boAutoGainSupported || !boGainAutoIsActive );

    //Blacklevel needs no refresh, the slider should always be visible since there is no auto-black-level functionality.
    pBtnGamma_->SetValue( settings.boGammaEnabled );

    if( settings.boColorEnabled )
    {
        const bool boAutoWhiteBalanceSupported = features.boAutoWhiteBalanceSupport;
        const bool boAutoWhiteBalanceIsActive = settings.boAutoWhiteBalanceEnabled;
        pBtnWhiteBalanceAuto_->SetValue( boAutoWhiteBalanceIsActive );
        pBtnWhiteBalanceAuto_->Enable( boAutoWhiteBalanceSupported );
        pSLWhiteBalanceR_->Enable( !boAutoWhiteBalanceSupported || !boAutoWhiteBalanceIsActive );
        pSCWhiteBalanceR_->Enable( !boAutoWhiteBalanceSupported || !boAutoWhiteBalanceIsActive );
        pSLWhiteBalanceB_->Enable( !boAutoWhiteBalanceSupported || !boAutoWhiteBalanceIsActive );
        pSCWhiteBalanceB_->Enable( !boAutoWhiteBalanceSupported || !boAutoWhiteBalanceIsActive );
    }
    else
    {
        pSLWhiteBalanceR_->Enable( false );
        pSCWhiteBalanceR_->Enable( false );
        pSLWhiteBalanceB_->Enable( false );
        pSCWhiteBalanceB_->Enable( false );
        pBtnWhiteBalanceAuto_->Enable( false );
        pBtnWhiteBalanceAuto_->SetValue( false );
    }

    if( settings.boColorEnabled )
    {
        const bool boCCMSupported = features.boColorOptionsSupport;
        const bool boCCMIsActive = settings.boCCMEnabled;
        pBtnCCM_->SetValue( boCCMIsActive );
        pBtnCCM_->Enable( boCCMSupported );
        pSLSaturation_->Enable( true );
        pSCSaturation_->Enable( true );
    }
    else
    {
        pSLSaturation_->Enable( false );
        pSCSaturation_->Enable( false );
        pBtnCCM_->Enable( false );
        pBtnCCM_->SetValue( false );
    }

    const bool boAutoFrameRateSupported = features.boAutoFrameRateSupport;
    const bool boFrameRateAutoIsActive = settings.boAutoFrameRateEnabled;
    const bool boFrameRateRegulationSupported = features.boRegulateFrameRateSupport;
    pBtnFrameRateAuto_->SetValue( boFrameRateAutoIsActive );
    pBtnFrameRateAuto_->Enable( boAutoFrameRateSupported );
    pSLFrameRate_->Enable( boFrameRateRegulationSupported ? ( !boAutoFrameRateSupported || !boFrameRateAutoIsActive ) : false );
    pSCFrameRate_->Enable( boFrameRateRegulationSupported ? ( !boAutoFrameRateSupported || !boFrameRateAutoIsActive ) : false );
    pFrameRateControlStaticText_->Enable( boFrameRateRegulationSupported );
    pBtnPresetColor_->Enable( features.boColorOptionsSupport );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::RestoreWizardConfiguration( void )
//-----------------------------------------------------------------------------
{
    DeviceSettings settings = currentSettings_[currentDeviceSerial_];

    if( boGUILocked_ == false )
    {
        pSCExposure_->SetValue( settings.exposureTime );
        HandleExposureSpinControlChanges();
        if( featuresSupported_[currentDeviceSerial_].boAutoExposureSupport )
        {
            ConfigureExposureAuto( settings.boAutoExposureEnabled );
        }

        pSCGain_->SetValue( settings.unifiedGain );
        HandleGainSpinControlChanges();
        if( featuresSupported_[currentDeviceSerial_].boAutoGainSupport )
        {
            ConfigureGainAuto( settings.boAutoGainEnabled );
        }

        pSCBlackLevel_->SetValue( settings.unifiedBlackLevel );
        HandleBlackLevelSpinControlChanges();
        ConfigureGamma( settings.boGammaEnabled );

        if( featuresSupported_[currentDeviceSerial_].boColorOptionsSupport )
        {
            pSCWhiteBalanceR_->SetValue( settings.whiteBalanceRed );
            HandleWhiteBalanceRSpinControlChanges();
            pSCWhiteBalanceB_->SetValue( settings.whiteBalanceBlue );
            HandleWhiteBalanceBSpinControlChanges();
            if( featuresSupported_[currentDeviceSerial_].boAutoWhiteBalanceSupport )
            {
                ConfigureWhiteBalanceAuto( settings.boAutoWhiteBalanceEnabled );
            }

            pSCSaturation_->SetValue( settings.saturation );
            HandleSaturationSpinControlChanges();
            ConfigureCCM( settings.boCCMEnabled );
        }

        pSCFrameRate_->SetValue( settings.frameRate );
        HandleFrameRateSpinControlChanges();
        if( featuresSupported_[currentDeviceSerial_].boAutoFrameRateSupport )
        {
            ConfigureFrameRateAuto( settings.boAutoFrameRateEnabled );
        }
    }

    RefreshControls();
    SetPixelFormat( settings.imageFormatControlPixelFormat );
    pID_->pixelFormat.writeS( settings.imageDestinationPixelFormat );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::SaveWizardConfiguration( void )
//-----------------------------------------------------------------------------
{
    currentSettings_[currentDeviceSerial_].exposureTime = pSCExposure_->GetValue();
    currentSettings_[currentDeviceSerial_].boAutoExposureEnabled = pBtnExposureAuto_->GetValue();
    currentSettings_[currentDeviceSerial_].unifiedGain = pSCGain_->GetValue();
    currentSettings_[currentDeviceSerial_].boAutoGainEnabled = pBtnGainAuto_->GetValue();
    currentSettings_[currentDeviceSerial_].unifiedBlackLevel = pSCBlackLevel_->GetValue();
    currentSettings_[currentDeviceSerial_].boGammaEnabled = pBtnGamma_->GetValue();
    currentSettings_[currentDeviceSerial_].whiteBalanceRed = pSCWhiteBalanceR_->GetValue();
    currentSettings_[currentDeviceSerial_].whiteBalanceBlue = pSCWhiteBalanceB_->GetValue();
    currentSettings_[currentDeviceSerial_].boAutoWhiteBalanceEnabled = pBtnWhiteBalanceAuto_->GetValue();
    currentSettings_[currentDeviceSerial_].saturation = pSCSaturation_->GetValue();
    currentSettings_[currentDeviceSerial_].boCCMEnabled = pBtnCCM_->GetValue();
    currentSettings_[currentDeviceSerial_].frameRate = pSCFrameRate_->GetValue();
    currentSettings_[currentDeviceSerial_].boAutoFrameRateEnabled = pBtnFrameRateAuto_->GetValue();
    currentSettings_[currentDeviceSerial_].imageFormatControlPixelFormat = GetPixelFormat();
    currentSettings_[currentDeviceSerial_].imageDestinationPixelFormat = pID_->pixelFormat.readS();
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::SelectLUTDependingOnPixelFormat( void )
//-----------------------------------------------------------------------------
{
    string currentPixelFormat = currentSettings_[currentDeviceSerial_].imageFormatControlPixelFormat;

    if( currentPixelFormat.find( "8", 0 ) != string::npos &&
        pIP_->LUTMappingSoftware.read() != LUTm8To8 )
    {
        pIP_->LUTMappingSoftware.write( LUTm8To8 );
    }
    else if( currentPixelFormat.find( "10", 0 ) != string::npos &&
             pIP_->LUTMappingSoftware.read() != LUTm10To10 )
    {
        pIP_->LUTMappingSoftware.write( LUTm10To10 );
    }
    else if( currentPixelFormat.find( "12", 0 ) != string::npos &&
             pIP_->LUTMappingSoftware.read() != LUTm12To12 )
    {
        pIP_->LUTMappingSoftware.write( LUTm12To12 );
    }
    else if( currentPixelFormat.find( "14", 0 ) != string::npos &&
             pIP_->LUTMappingSoftware.read() != LUTm14To14 )
    {
        pIP_->LUTMappingSoftware.write( LUTm14To14 );
    }
    else if( currentPixelFormat.find( "16", 0 ) != string::npos &&
             pIP_->LUTMappingSoftware.read() != LUTm16To16 )
    {
        pIP_->LUTMappingSoftware.write( LUTm16To16 );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::SetupControls( void )
//-----------------------------------------------------------------------------
{
    try
    {
        SetupExposureControls();
        SetupGainControls();
        SetupBlackLevelControls();
        SetupWhiteBalanceControls();

        if( featuresSupported_[currentDeviceSerial_].boColorOptionsSupport )
        {
            const double saturationRangeMin = 0.;
            const double saturationRangeMax = 200.;
            const double saturation = ReadSaturationData();

            VarScopeMod<bool> scopeGUILocked( boGUILocked_, false ); // unfortunately some of the next lines emit messages which cause message handlers to be invoked. This needs to be blocked here
            boGUILocked_ = true;
            pSLSaturation_->SetRange( static_cast< int >( saturationRangeMin * SLIDER_GRANULARITY_ ), static_cast< int >( saturationRangeMax * SLIDER_GRANULARITY_ ) );
            pSCSaturation_->SetRange( saturationRangeMin, saturationRangeMax );
            pSCSaturation_->SetIncrement( 1 / SLIDER_GRANULARITY_ );
            pSCSaturation_->SetValue( saturation );
            pSLSaturation_->SetValue( static_cast< int >( saturation * SLIDER_GRANULARITY_ ) );
        }
        else
        {
            // This has to be done for aesthetic reasons. If a grayscale camera is opened, the saturation control is
            // of course grayed out, however the last value (e.g. from the previous color camera) is still being shown
            pSCSaturation_->SetValue( 100. );
            pSLSaturation_->SetValue( 100. );
        }
        SetupFrameRateControls();
    }
    catch( const ImpactAcquireException& ) {}
    RefreshControls();
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::SetupDevice( void )
//-----------------------------------------------------------------------------
{
    SupportedWizardFeatures features = featuresSupported_[currentDeviceSerial_];
    DeviceSettings devSettings = currentSettings_[currentDeviceSerial_];

    try
    {
        InitializeExposureParameters( devSettings, features );
        InitializeGainParameters( devSettings, features );
        InitializeBlackLevelParameters( devSettings, features );
        WriteUnifiedBlackLevelData( 0. );

        if( features.boColorOptionsSupport )
        {
            InitializeWhiteBalanceParameters(  devSettings, features );
        }

        if( features.boAutoFrameRateSupport )
        {
            SetAcquisitionFrameRateLimitMode();
            devSettings.boAutoFrameRateEnabled = true;
        }
        else
        {
            //By entering a big number the maxValue of the Framerate will be written
            SetFrameRate( 10000. );
            devSettings.boAutoFrameRateEnabled = false;
        }
    }
    catch( const ImpactAcquireException& e )
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Error during device Setup (Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
    }
    currentSettings_[currentDeviceSerial_] = devSettings;
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::SetupDriverSettings( void )
//-----------------------------------------------------------------------------
{
    if( pIP_->LUTEnable.isValid() && pIP_->LUTEnable.isWriteable() )
    {
        pIP_->LUTEnable.write( bTrue );
        pIP_->LUTMode.write( LUTmGamma );
        const unsigned int LUTCnt = pIP_->getLUTParameterCount();
        for( unsigned int i = 0; i < LUTCnt; i++ )
        {
            mvIMPACT::acquire::LUTParameters& lpm = pIP_->getLUTParameter( i );
            lpm.gamma.write( GAMMA_CORRECTION_VALUE_ );
            lpm.gammaMode.write( LUTgmLinearStart );
        }
        pIP_->LUTEnable.write( bFalse );
    }

    if( featuresSupported_[currentDeviceSerial_].boColorOptionsSupport )
    {
        if( pIP_->colorTwistEnable.isValid() && pIP_->colorTwistEnable.isWriteable() )
        {
            pIP_->colorTwistEnable.write( bTrue );
        }
        if( ( pIP_->colorTwistInputCorrectionMatrixEnable.isValid() && pIP_->colorTwistInputCorrectionMatrixEnable.isWriteable() ) &&
            ( pIP_->colorTwistOutputCorrectionMatrixEnable.isValid() && pIP_->colorTwistOutputCorrectionMatrixEnable.isWriteable() ) )
        {
            pIP_->colorTwistInputCorrectionMatrixEnable.write( bTrue );
            pIP_->colorTwistOutputCorrectionMatrixEnable.write( bTrue );
            pIP_->colorTwistInputCorrectionMatrixMode.write( cticmmDeviceSpecific );
            pIP_->colorTwistOutputCorrectionMatrixMode.write( ctocmmXYZTosRGB_D50 );
            pIP_->colorTwistInputCorrectionMatrixEnable.write( bFalse );
            pIP_->colorTwistOutputCorrectionMatrixEnable.write( bFalse );
        }
    }
    pID_->pixelFormat.writeS( currentSettings_[currentDeviceSerial_].imageDestinationPixelFormat );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::SetupUnifiedData( bool boNewDevice )
//-----------------------------------------------------------------------------
{
    if( boNewDevice )
    {
        SetupUnifiedGainData();
        SetupUnifiedBlackLevelData();
    }
    DeviceSettings devSettings = currentSettings_[currentDeviceSerial_];
    analogGainMin_ = devSettings.analogGainMin;
    analogGainMax_ = devSettings.analogGainMax;
    digitalGainMin_ = devSettings.digitalGainMin;
    digitalGainMax_ = devSettings.digitalGainMax;
    analogBlackLevelMin_ = devSettings.analogBlackLevelMin;
    analogBlackLevelMax_ = devSettings.analogBlackLevelMax;
    digitalBlackLevelMin_ = devSettings.digitalBlackLevelMin;
    digitalBlackLevelMax_ = devSettings.digitalBlackLevelMax;
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::SetAcquisitionFrameRateLimitMode( void )
//-----------------------------------------------------------------------------
{
    DoSetAcquisitionFrameRateLimitMode();
}

//-----------------------------------------------------------------------------
bool WizardQuickSetup::ShowFactoryResetPopup( void )
//-----------------------------------------------------------------------------
{
    return wxMessageBox( wxT( "Are you sure you want to load the factory settings?\nAll current settings will be overwritten!\n\nThis cannot be undone by pressing 'Cancel'." ), wxT( "About to load factory settings" ), wxYES_NO | wxNO_DEFAULT | wxICON_EXCLAMATION, this ) == wxYES;
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::ShowImageTimeoutPopup( void )
//-----------------------------------------------------------------------------
{
    wxMessageBox( wxT( "The last Image Request returned with an Image Timeout.\nThis means that the camera cannot stream and indicates a problem with the current configuration.\n\nPlease press the 'Factory' button to load the Factory Settings and then continue with the setup." ), wxT( "Image Timeout!" ), wxOK | wxICON_INFORMATION, this );
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::UpdateExposureControlsFromCamera( void )
//-----------------------------------------------------------------------------
{
    double const exposureTime = GetExposureTime();
    pSCExposure_->SetValue( exposureTime );
    pSLExposure_->SetValue( ExposureToSliderValue( exposureTime ) );
    currentSettings_[currentDeviceSerial_].exposureTime = exposureTime;
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::UpdateGainControlsFromCamera( void )
//-----------------------------------------------------------------------------
{
    // ReadUnifiedGain leads to stuttering. Since DigitalGain is set to 0 on pressing AutoGain Button,
    // one could rely on analog gain values alone to update the current gain when autogain is in use.
    // This option however has to be investigated further before using.
    // double const unifiedGain = pAnC_->gain.read();
    double const unifiedGain = ReadUnifiedGainData();
    pSCGain_->SetValue( unifiedGain );
    pSLGain_->SetValue( static_cast< int >( unifiedGain * SLIDER_GRANULARITY_ ) );
    currentSettings_[currentDeviceSerial_].unifiedGain = unifiedGain;
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::UpdateWhiteBalanceControlsFromCamera( void )
//-----------------------------------------------------------------------------
{
    double const whiteBalanceRed = GetWhiteBalance( wbcRed ) * 100.;
    double const whiteBalanceBlue = GetWhiteBalance( wbcBlue ) * 100.;
    pSCWhiteBalanceR_->SetValue( whiteBalanceRed );
    pSLWhiteBalanceR_->SetValue( static_cast< int >( whiteBalanceRed * SLIDER_GRANULARITY_ ) );
    currentSettings_[currentDeviceSerial_].whiteBalanceRed = whiteBalanceRed;
    pSCWhiteBalanceB_->SetValue( whiteBalanceBlue );
    pSLWhiteBalanceB_->SetValue( static_cast< int >( whiteBalanceBlue * SLIDER_GRANULARITY_ ) );
    currentSettings_[currentDeviceSerial_].whiteBalanceBlue = whiteBalanceBlue;
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::UpdateControlsData( void )
//-----------------------------------------------------------------------------
{
    if( featuresSupported_[currentDeviceSerial_].boAutoExposureSupport &&
        currentSettings_[currentDeviceSerial_].boAutoExposureEnabled )
    {
        UpdateExposureControlsFromCamera();
    }
    if( featuresSupported_[currentDeviceSerial_].boAutoGainSupport &&
        currentSettings_[currentDeviceSerial_].boAutoGainEnabled )
    {
        UpdateGainControlsFromCamera();
    }
    if( ( featuresSupported_[currentDeviceSerial_].boAutoWhiteBalanceSupport &&
          currentSettings_[currentDeviceSerial_].boAutoWhiteBalanceEnabled &&
          currentSettings_[currentDeviceSerial_].boColorEnabled ) ||
        pDev_->family.readS() == "mvBlueFOX" )
    {
        UpdateWhiteBalanceControlsFromCamera();
    }
    if( featuresSupported_[currentDeviceSerial_].boAutoFrameRateSupport &&
        currentSettings_[currentDeviceSerial_].boAutoFrameRateEnabled )
    {
        SetupFrameRateControls();
        currentSettings_[currentDeviceSerial_].frameRate = pSCFrameRate_->GetMax();
    }
    RefreshControls();
}

//////////////////////////STANDARD DIALOG-BUTTONS//////////////////////////////
//-----------------------------------------------------------------------------
void WizardQuickSetup::OnBtnCancel( wxCommandEvent& )
//-----------------------------------------------------------------------------
{
    CloseDlg();
}

//-----------------------------------------------------------------------------
void WizardQuickSetup::OnBtnOk( wxCommandEvent& )
//-----------------------------------------------------------------------------
{
    SaveWizardConfiguration();
    pParentPropViewFrame_->RestoreGUIStateAfterQuickSetupWizard();
    Hide();
}

//=============================================================================
//================= Implementation WizardQuickSetupGenICam ====================
//=============================================================================
//-----------------------------------------------------------------------------
WizardQuickSetupGenICam::WizardQuickSetupGenICam( PropViewFrame* pParent, const wxString& title, bool boShowAtStartup ) :
    WizardQuickSetup( pParent, title, boShowAtStartup ), pAcC_( 0 ), pAnC_( 0 ), pIFC_( 0 ), pUSC_( 0 )
//-----------------------------------------------------------------------------
{

}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::CreateInterfaceLayoutSpecificControls( Device* pDev )
//-----------------------------------------------------------------------------
{
    pAcC_ = new GenICam::AcquisitionControl( pDev );
    pAnC_ = new GenICam::AnalogControl( pDev );
    pIFC_ = new GenICam::ImageFormatControl( pDev );
    pUSC_ = new GenICam::UserSetControl( pDev );
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::DeleteInterfaceLayoutSpecificControls( void )
//-----------------------------------------------------------------------------
{
    DeleteElement( pAcC_ );
    DeleteElement( pAnC_ );
    DeleteElement( pIFC_ );
    DeleteElement( pUSC_ );
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::DoConfigureFrameRateAuto( bool boActive, double frameRateValue )
//-----------------------------------------------------------------------------
{
    if( boActive )
    {
        currentSettings_[currentDeviceSerial_].frameRate = frameRateValue;
        if( pAcC_->acquisitionFrameRate.hasMaxValue() )
        {
            pAcC_->acquisitionFrameRate.write( pAcC_->acquisitionFrameRate.getMaxValue() );
        }
        SetFrameRateEnable( false );
    }
    else
    {
        SetFrameRateEnable( true );
        if( pAcC_->acquisitionFrameRate.hasMaxValue() )
        {
            //In the case of FrameRate, checks have to be done first, as due to changes in exposure, the desired value
            //may be out of range (e.g. writing the maximum framerate value after exposure has increased dramatically)
            double frameRateValue = currentSettings_[currentDeviceSerial_].frameRate;
            double frameRateMin = pAcC_->acquisitionFrameRate.getMinValue();
            double frameRateMax = pAcC_->acquisitionFrameRate.getMaxValue();
            if( frameRateValue <= frameRateMin )
            {
                pAcC_->acquisitionFrameRate.write( frameRateMin );
                currentSettings_[currentDeviceSerial_].frameRate = frameRateMin;
            }
            else if( frameRateValue >= frameRateMax )
            {
                pAcC_->acquisitionFrameRate.write( frameRateMax );
                currentSettings_[currentDeviceSerial_].frameRate = frameRateMax;
            }
            else
            {
                pAcC_->acquisitionFrameRate.write( frameRateValue );
            }
        }
        SetupFrameRateControls();
    }
}

//-----------------------------------------------------------------------------
double WizardQuickSetupGenICam::DoReadUnifiedBlackLevel( void ) const
//-----------------------------------------------------------------------------
{
    //const string originalSetting = pAnC_->blackLevelSelector.readS();
    pAnC_->blackLevelSelector.writeS( "All" );
    double currentBlackLevel = pAnC_->blackLevel.read();
    pAnC_->blackLevelSelector.writeS( "DigitalAll" );
    currentBlackLevel += pAnC_->blackLevel.read();
    //pAnC_->blackLevelSelector.writeS( originalSetting );
    return currentBlackLevel;
}

//-----------------------------------------------------------------------------
double WizardQuickSetupGenICam::DoReadUnifiedGain( void ) const
//-----------------------------------------------------------------------------
{
    //const string originalSetting = pAnC_->gainSelector.readS();
    pAnC_->gainSelector.writeS( "AnalogAll" );
    double currentGain = pAnC_->gain.read();
    pAnC_->gainSelector.writeS( "DigitalAll" );
    currentGain += pAnC_->gain.read();
    //pAnC_->gainSelector.writeS( originalSetting );
    return currentGain;
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::DoSetAcquisitionFrameRateLimitMode( void )
//-----------------------------------------------------------------------------
{
    pAcC_->mvAcquisitionFrameRateLimitMode.writeS( "mvDeviceLinkThroughput" );
    SetFrameRateEnable( true );
    if( pAcC_->acquisitionFrameRate.hasMaxValue() )
    {
        pAcC_->acquisitionFrameRate.write( pAcC_->acquisitionFrameRate.getMaxValue() );
    }
    SetFrameRateEnable( false );
    currentSettings_[currentDeviceSerial_].boAutoFrameRateEnabled = true;
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::DoWriteUnifiedBlackLevelData( double value )
//-----------------------------------------------------------------------------
{
    try
    {
        //const string originalSetting = pAnC_->blackLevelSelector.readS();
        pAnC_->blackLevelSelector.writeS( "All" );
        if( value >= analogBlackLevelMin_ && value <= analogBlackLevelMax_ )
        {
            pAnC_->blackLevel.write( value );
        }
        else if( value < analogBlackLevelMin_ )
        {
            pAnC_->blackLevel.write( analogBlackLevelMin_ );
        }
        else if( value > analogBlackLevelMax_ )
        {
            pAnC_->blackLevel.write( analogBlackLevelMax_ );
        }
        double diff = value - pAnC_->blackLevel.read();
        pAnC_->blackLevelSelector.writeS( "DigitalAll" );
        pAnC_->blackLevel.write( diff );
        //pAnC_->blackLevelSelector.writeS( originalSetting );
    }
    catch( const ImpactAcquireException& e )
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to write combined analog and digital blackLevel values:\n%s(%s)" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::DoWriteUnifiedGain( double value ) const
//-----------------------------------------------------------------------------
{
    //const string originalSetting = pAnC_->gainSelector.readS();
    pAnC_->gainSelector.writeS( "AnalogAll" );
    if( ( value >= analogGainMin_ ) && ( value <= analogGainMax_ ) )
    {
        pAnC_->gain.write( value );
    }
    else if( value < analogGainMin_ )
    {
        pAnC_->gain.write( analogGainMin_ );
    }
    else if( value > analogGainMax_ )
    {
        pAnC_->gain.write( analogGainMax_ );
    }
    double diff = value - pAnC_->gain.read();
    pAnC_->gainSelector.writeS( "DigitalAll" );
    pAnC_->gain.write( diff );
    //pAnC_->gainSelector.writeS( originalSetting );
}

//-----------------------------------------------------------------------------
string WizardQuickSetupGenICam::GetPixelFormat( void ) const
//-----------------------------------------------------------------------------
{
    return ( pIFC_->pixelFormat.isValid() ) ? pIFC_->pixelFormat.readS() : string( "Mono8" );
}

//-----------------------------------------------------------------------------
double WizardQuickSetupGenICam::GetWhiteBalance( TWhiteBalanceChannel channel )
//-----------------------------------------------------------------------------
{
    pAnC_->balanceRatioSelector.writeS( ( channel == wbcRed ) ? "Red" : "Blue" );
    return pAnC_->balanceRatio.read();
}

//-----------------------------------------------------------------------------
bool WizardQuickSetupGenICam::HasAEC( void ) const
//-----------------------------------------------------------------------------
{
    if( pAcC_->exposureAuto.isValid() && pAcC_->exposureAuto.isWriteable() )
    {
        vector<string> validExposureAutoValues;
        pAcC_->exposureAuto.getTranslationDictStrings( validExposureAutoValues );
        const vector<string>::const_iterator it = validExposureAutoValues.begin();
        const vector<string>::const_iterator itEND = validExposureAutoValues.end();
        if( find( it, itEND, "Continuous" ) != itEND )
        {
            if( pAcC_->mvExposureAutoMode.isValid() )
            {
                vector<string> mvExposureAutoModes;
                pAcC_->mvExposureAutoMode.getTranslationDictStrings( mvExposureAutoModes );
                vector<string>::iterator itEND = mvExposureAutoModes.end();
                if( find( mvExposureAutoModes.begin(), itEND, "mvDevice" ) != itEND )
                {
                    return true;
                }
            }
            else
            {
                return true;
            }
        }
    }
    return false;
}

//-----------------------------------------------------------------------------
bool WizardQuickSetupGenICam::HasAGC( void )
//-----------------------------------------------------------------------------
{
    vector<string> gainSelectorStrings;
    pAnC_->gainSelector.getTranslationDictStrings( gainSelectorStrings );
    const vector<string>::const_iterator it = gainSelectorStrings.begin();
    const vector<string>::const_iterator itEND = gainSelectorStrings.end();
    const string originalSetting = pAnC_->gainSelector.readS();
    if( find( it, itEND, "AnalogAll" ) != itEND )
    {
        pAnC_->gainSelector.writeS( "AnalogAll" );
        if( pAnC_->gainAuto.isValid() && pAnC_->gainAuto.isWriteable() )
        {
            if( pAnC_->mvGainAutoMode.isValid() )
            {
                vector<string> mvGainAutoModes;
                pAnC_->mvGainAutoMode.getTranslationDictStrings( mvGainAutoModes );
                vector<string>::iterator itEND = mvGainAutoModes.end();
                if( find( mvGainAutoModes.begin(), itEND, "mvDevice" ) != itEND )
                {
                    return true;
                }
            }
            else
            {
                return true;
            }
        }
    }
    else
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Device does not seem to have a AnalogAll Selector!" ) ) );
    }
    pAnC_->gainSelector.writeS( originalSetting );
    return false;
}

//-----------------------------------------------------------------------------
bool WizardQuickSetupGenICam::HasAutoFrameRate( void ) const
//-----------------------------------------------------------------------------
{
    return pAcC_->mvAcquisitionFrameRateLimitMode.isValid() && pAcC_->mvAcquisitionFrameRateLimitMode.isWriteable();
}

//-----------------------------------------------------------------------------
bool WizardQuickSetupGenICam::HasAWB( void ) const
//-----------------------------------------------------------------------------
{
    if( pAnC_->balanceWhiteAuto.isValid() && pAnC_->balanceWhiteAuto.isWriteable() )
    {
        vector<string> validWhiteBalanceAutoValues;
        pAnC_->balanceWhiteAuto.getTranslationDictStrings( validWhiteBalanceAutoValues );
        const vector<string>::const_iterator it = validWhiteBalanceAutoValues.begin();
        const vector<string>::const_iterator itEND = validWhiteBalanceAutoValues.end();
        if( find( it, itEND, "Continuous" ) != itEND )
        {
            return true;
        }
    }
    return false;
}

//-----------------------------------------------------------------------------
bool WizardQuickSetupGenICam::HasColorFormat( void ) const
//-----------------------------------------------------------------------------
{
    vector<string> pixelFormatStrings;
    pIFC_->pixelFormat.getTranslationDictStrings( pixelFormatStrings );
    const vector<string>::const_iterator itBEGIN = pixelFormatStrings.begin();
    const vector<string>::const_iterator itEND = pixelFormatStrings.end();
    for( vector<string>::const_iterator it = itBEGIN; it != itEND; ++it )
    {
        if( it->find( "GB", 0 ) != string::npos ||
            it->find( "BG", 0 ) != string::npos ||
            it->find( "GR", 0 ) != string::npos ||
            it->find( "RG", 0 ) != string::npos )
        {
            return true;
        }
    }
    return false;
}

//-----------------------------------------------------------------------------
bool WizardQuickSetupGenICam::HasFactoryDefault( void ) const
//-----------------------------------------------------------------------------
{
    if( pUSC_->userSetSelector.isValid() && pUSC_->userSetLoad.isValid() )
    {
        vector<string> validUserSets;
        pUSC_->userSetSelector.getTranslationDictStrings( validUserSets );
        if( find( validUserSets.begin(), validUserSets.end(), string( "Default" ) ) != validUserSets.end() )
        {
            return true;
        }
    }
    return false;
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::InitializeExposureParameters( DeviceSettings& devSettings, const SupportedWizardFeatures& features )
//-----------------------------------------------------------------------------
{
    if( features.boAutoExposureSupport )
    {
        pAcC_->exposureAuto.writeS( string( "Continuous" ) );
        pAcC_->mvExposureAutoUpperLimit.write( 200000. );
        pAcC_->mvExposureAutoAverageGrey.write( 50 );
        if( pAcC_->mvExposureAutoMode.isValid() )
        {
            pAcC_->mvExposureAutoMode.writeS( "mvDevice" );
        }
        devSettings.boAutoExposureEnabled = true;
    }
    else
    {
        SetExposureTime( 25000 );
        devSettings.boAutoExposureEnabled = false;
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::InitializeGainParameters( DeviceSettings& devSettings, const SupportedWizardFeatures& features )
//-----------------------------------------------------------------------------
{
    if( features.boAutoGainSupport )
    {
        DoWriteUnifiedGain( 0. );
        pAnC_->gainSelector.writeS( string( "AnalogAll" ) );
        pAnC_->gainAuto.writeS( string( "Continuous" ) );
        devSettings.boAutoGainEnabled = true;
    }
    else
    {
        DoWriteUnifiedGain( 0. );
        devSettings.boAutoGainEnabled = false;
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::InitializeBlackLevelParameters( DeviceSettings& /*devSettings*/, const SupportedWizardFeatures& /*features*/ )
//-----------------------------------------------------------------------------
{
    DoWriteUnifiedBlackLevelData( 0. );
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::InitializeWhiteBalanceParameters( DeviceSettings& devSettings, const SupportedWizardFeatures& features )
//-----------------------------------------------------------------------------
{
    if( features.boAutoWhiteBalanceSupport )
    {
        SetAutoWhiteBalance( true );
        devSettings.boAutoWhiteBalanceEnabled = true;
    }
    else
    {
        SetWhiteBalance( wbcRed, 1.4 );
        SetWhiteBalance( wbcBlue, 2. );
        devSettings.boAutoWhiteBalanceEnabled = false;
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::QueryInterfaceLayoutSpecificSettings( DeviceSettings& devSettings )
//-----------------------------------------------------------------------------
{
    if( pAcC_->exposureTime.isValid() )
    {
        devSettings.exposureTime = pAcC_->exposureTime.read();
    }
    if( pAcC_->exposureAuto.isValid() )
    {
        devSettings.boAutoExposureEnabled = pAcC_->exposureAuto.readS() != string( "Off" );
    }

    if( pAnC_->gainAuto.isValid() )
    {
        devSettings.boAutoGainEnabled = pAnC_->gainAuto.readS() != string( "Off" );
    }

    if( pAnC_->blackLevelSelector.isValid() )
    {
        devSettings.unifiedBlackLevel = DoReadUnifiedBlackLevel();
    }

    if( pAnC_->balanceRatioSelector.isValid() )
    {
        pAnC_->balanceRatioSelector.writeS( "Red" );
        devSettings.whiteBalanceRed = pAnC_->balanceRatio.read() * 100.;
        pAnC_->balanceRatioSelector.writeS( "Blue" );
        devSettings.whiteBalanceBlue = pAnC_->balanceRatio.read() * 100.;
    }
    if( pAnC_->balanceWhiteAuto.isValid() )
    {
        devSettings.boAutoWhiteBalanceEnabled = pAnC_->balanceWhiteAuto.readS() != string( "Off" );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::RestoreFactoryDefault( void )
//-----------------------------------------------------------------------------
{
    pUSC_->userSetSelector.writeS( "Default" );
    pUSC_->userSetLoad.call();
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::SelectColorPixelFormat( void )
//-----------------------------------------------------------------------------
{
    static const string rg = string( "BayerRG10" );
    static const string gr = string( "BayerGR10" );
    static const string bg = string( "BayerBG10" );
    static const string gb = string( "BayerGB10" );
    static const string defaultValue = string( "RGB8" );

    vector<string> pixelFormatStrings;
    pIFC_->pixelFormat.getTranslationDictStrings( pixelFormatStrings );
    const vector<string>::const_iterator itBEGIN = pixelFormatStrings.begin();
    const vector<string>::const_iterator itEND = pixelFormatStrings.end();
    if( find( itBEGIN, itEND, rg ) != itEND )
    {
        pIFC_->pixelFormat.writeS( rg );
        currentSettings_[currentDeviceSerial_].imageFormatControlPixelFormat = rg;
    }
    else if( find( itBEGIN, itEND, gr ) != itEND )
    {
        pIFC_->pixelFormat.writeS( gr );
        currentSettings_[currentDeviceSerial_].imageFormatControlPixelFormat = gr;
    }
    else if( find( itBEGIN, itEND, bg ) != itEND )
    {
        pIFC_->pixelFormat.writeS( bg );
        currentSettings_[currentDeviceSerial_].imageFormatControlPixelFormat = bg;
    }
    else if( find( itBEGIN, itEND, gb ) != itEND )
    {
        pIFC_->pixelFormat.writeS( gb );
        currentSettings_[currentDeviceSerial_].imageFormatControlPixelFormat = gb;
    }
    else
    {
        pIFC_->pixelFormat.writeS( defaultValue );
        currentSettings_[currentDeviceSerial_].imageFormatControlPixelFormat = defaultValue;
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::SelectGreyscalePixelFormat( void )
//-----------------------------------------------------------------------------
{
    if( !featuresSupported_[currentDeviceSerial_].boColorOptionsSupport )
    {
        const string mono = string( "Mono10" );
        const string defaultValue = string( "Mono8" );

        vector<string> pixelFormatStrings;
        pIFC_->pixelFormat.getTranslationDictStrings( pixelFormatStrings );
        vector<string>::iterator itEND = pixelFormatStrings.end();
        if( find( pixelFormatStrings.begin(), itEND, mono ) != itEND )
        {
            pIFC_->pixelFormat.writeS( mono );
            currentSettings_[currentDeviceSerial_].imageFormatControlPixelFormat = mono;
        }
        else
        {
            pIFC_->pixelFormat.writeS( "Mono8" );
            currentSettings_[currentDeviceSerial_].imageFormatControlPixelFormat = defaultValue;
        }
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::SetupExposureControls( void )
//-----------------------------------------------------------------------------
{
    if( pAcC_->exposureTime.isValid() )
    {
        const double exposureMin = pAcC_->exposureTime.hasMinValue() ? pAcC_->exposureTime.getMinValue() : 1.;
        const double exposureMax = ( pAcC_->exposureTime.hasMaxValue() && ( pAcC_->exposureTime.getMaxValue() < 200000. ) ) ? pAcC_->exposureTime.getMaxValue() : 200000.;
        const double exposure = pAcC_->exposureTime.read();
        const bool boHasStepWidth = pAcC_->exposureTime.hasStepWidth();
        double increment = 1.;
        if( boHasStepWidth )
        {
            increment = pAcC_->exposureTime.getStepWidth();
        }
        DoSetupExposureControls( exposureMin, exposureMax, exposure, boHasStepWidth, increment );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::SetupGainControls( void )
//-----------------------------------------------------------------------------
{
    if( pAnC_->gainSelector.isValid() )
    {
        const double gainUnifiedRangeMin = analogGainMin_ + digitalGainMin_;
        const double gainUnifiedRangeMax = analogGainMax_ + digitalGainMax_;
        const double gain = DoReadUnifiedGain();
        DoSetupGainControls( gainUnifiedRangeMin, gainUnifiedRangeMax, gain );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::SetupBlackLevelControls( void )
//-----------------------------------------------------------------------------
{
    if( pAnC_->blackLevelSelector.isValid() )
    {
        const double blackLevelUnifiedRangeMin = -24.;
        const double blackLevelUnifiedRangeMax = 24.;
        const double blackLevel = DoReadUnifiedBlackLevel();
        DoSetupBlackLevelControls( blackLevelUnifiedRangeMin, blackLevelUnifiedRangeMax, blackLevel );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::SetupWhiteBalanceControls( void )
//-----------------------------------------------------------------------------
{
    if( pAnC_->balanceRatioSelector.isValid() )
    {
        pAnC_->balanceRatioSelector.writeS( "Red" );
        const double whiteBalanceRMin = 1.;
        const double whiteBalanceRMax = 500.;
        const double whiteBalanceR = pAnC_->balanceRatio.read() * 100.0;
        pAnC_->balanceRatioSelector.writeS( "Blue" );
        const double whiteBalanceBMin = 1.;
        const double whiteBalanceBMax = 500.;
        const double whiteBalanceB = pAnC_->balanceRatio.read() * 100.0;
        DoSetupWhiteBalanceControls( whiteBalanceRMin, whiteBalanceRMax, whiteBalanceR, whiteBalanceBMin, whiteBalanceBMax, whiteBalanceB );

    }
    else
    {
        // This has to be done for aesthetic reasons. If a grayscale camera is opened, the whitebalance controls are
        // of course grayed out, however the last values (e.g. from the previous color camera) are still being shown
        DoSetupWhiteBalanceControls( 1., 500., 100., 1., 500., 100. );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::SetupFrameRateControls( void )
//-----------------------------------------------------------------------------
{
    if( pAcC_->mvAcquisitionFrameRateLimitMode.isValid() &&
        pAcC_->mvAcquisitionFrameRateLimitMode.readS() == string( "mvDeviceLinkThroughput" ) )
    {
        double frameRateRangeMin;
        double frameRateRangeMax;
        double frameRate;

        if( GetFrameRateEnable() )
        {
            // These settings are used when FrameRate is manually configured
            frameRateRangeMin = pAcC_->acquisitionFrameRate.hasMinValue() ? ( pAcC_->acquisitionFrameRate.getMinValue() >= 5. ? pAcC_->acquisitionFrameRate.getMinValue() : 5. ) : 2.;
            frameRateRangeMax = pAcC_->acquisitionFrameRate.hasMaxValue() ? pAcC_->acquisitionFrameRate.getMaxValue() : 100.;
            frameRate = pAcC_->acquisitionFrameRate.read();
        }
        else
        {
            // These settings are used when FrameRate is set to Auto.
            //--------------------------------------
            // Hardcoding the frameRateRangeMin value is not elegant; however there is no other obvious minimum
            // that could be used instead. Number 5 also fits with the maximum exposure limit of the wizard (200ms).
            frameRateRangeMin = 5;
            frameRateRangeMax = pAcC_->mvResultingFrameRate.isValid() ? pAcC_->mvResultingFrameRate.read() : 100.;
            frameRate = pAcC_->mvResultingFrameRate.read();
        }
        DoSetupFrameRateControls( frameRateRangeMin, frameRateRangeMax, frameRate );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::SetWhiteBalance( TWhiteBalanceChannel channel, double value )
//-----------------------------------------------------------------------------
{
    pAnC_->balanceRatioSelector.writeS( ( channel == wbcRed ) ? "Red" : "Blue" );
    pAnC_->balanceRatio.write( value );
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::SetFrameRate( double value )
//-----------------------------------------------------------------------------
{
    if( pAcC_->acquisitionFrameRate.hasMaxValue() )
    {
        //Out-of-bounds check because of Framerate controls' dynamic nature
        double currentMaxFrameRate = pAcC_->acquisitionFrameRate.getMaxValue();
        if( value > currentMaxFrameRate )
        {
            pAcC_->acquisitionFrameRate.write( currentMaxFrameRate );
        }
        else
        {
            pAcC_->acquisitionFrameRate.write( value );
        }
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::SetAutoGain( bool boEnable )
//-----------------------------------------------------------------------------
{
    string originalSelection = pAnC_->gainSelector.readS();
    if( originalSelection != "AnalogAll" )
    {
        pAnC_->gainSelector.writeS( "AnalogAll" );
    }
    pAnC_->gainAuto.writeS( string( boEnable ? "Continuous" : "Off" ) );
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::SetupUnifiedGainData( void )
//-----------------------------------------------------------------------------
{
    vector<string> gainSelectorStrings;
    pAnC_->gainSelector.getTranslationDictStrings( gainSelectorStrings );
    vector<string>::iterator it = gainSelectorStrings.begin();
    vector<string>::iterator itEND = gainSelectorStrings.end();
    string originalSetting = pAnC_->gainSelector.readS();
    if( find( it, itEND, "AnalogAll" ) != itEND && find( it, itEND, "DigitalAll" ) != itEND )
    {
        pAnC_->gainSelector.writeS( "AnalogAll" );
        currentSettings_[currentDeviceSerial_].analogGainMax = pAnC_->gain.getMaxValue();
        currentSettings_[currentDeviceSerial_].analogGainMin = ( pAnC_->gain.getMinValue() < 0 ) ? ( pAnC_->gain.getMinValue() < -6.0 ) ? ( -6.0 ) : ( pAnC_->gain.getMinValue() ) : 0;
        pAnC_->gainSelector.writeS( "DigitalAll" );
        currentSettings_[currentDeviceSerial_].digitalGainMax = pAnC_->gain.getMaxValue();
        currentSettings_[currentDeviceSerial_].digitalGainMin = pAnC_->gain.getMinValue();
    }
    else
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Device does not support an 'AnalogAll' or a 'DigitalAll' Gain selector!" ) ) );
    }
    pAnC_->gainSelector.writeS( originalSetting );
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::SetupUnifiedBlackLevelData( void )
//-----------------------------------------------------------------------------
{
    vector<string> blackLevelSelectorStrings;
    pAnC_->blackLevelSelector.getTranslationDictStrings( blackLevelSelectorStrings );
    vector<string>::iterator it = blackLevelSelectorStrings.begin();
    vector<string>::iterator itEND = blackLevelSelectorStrings.end();
    string originalSetting = pAnC_->blackLevelSelector.readS();
    if( find( it, itEND, "All" ) != itEND && find( it, itEND, "DigitalAll" ) != itEND )
    {
        pAnC_->blackLevelSelector.writeS( "All" );
        currentSettings_[currentDeviceSerial_].analogBlackLevelMax = pAnC_->blackLevel.getMaxValue();
        currentSettings_[currentDeviceSerial_].analogBlackLevelMin = ( pAnC_->blackLevel.getMinValue() < 0 ) ? ( pAnC_->blackLevel.getMinValue() < -6.0 ) ? ( -6.0 ) : ( pAnC_->blackLevel.getMinValue() ) : 0;
        pAnC_->blackLevelSelector.writeS( "DigitalAll" );
        currentSettings_[currentDeviceSerial_].digitalBlackLevelMax = pAnC_->blackLevel.getMaxValue();
        currentSettings_[currentDeviceSerial_].digitalBlackLevelMin = pAnC_->blackLevel.getMinValue();
    }
    else
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Device does not support an 'All' or a 'DigitalAll' BlackLevel selector!" ) ) );
    }
    pAnC_->blackLevelSelector.writeS( originalSetting );
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::TryToReadFrameRate( double& value )
//-----------------------------------------------------------------------------
{
    if( pAcC_->acquisitionFrameRate.isValid() && pAcC_->mvAcquisitionFrameRateLimitMode.isValid() )
    {
        // Up to this point the device has not been analysed and there is no information available about
        // what capabilities it supports, and crucially whether it supports Auto framrate or not. Maybe there
        // is another way to do this, or the sequence of QueryInitialSettings and AnalyzeDeviceAndGatherInformation
        // should be reworked...? As of now it is not clear what will happen with third-party devices.
        pAcC_->mvAcquisitionFrameRateLimitMode.writeS( "mvDeviceLinkThroughput" );
        const bool boPreviousSetting = GetFrameRateEnable();
        SetFrameRateEnable( true );
        value = pAcC_->acquisitionFrameRate.read();
        SetFrameRateEnable( boPreviousSetting );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::WriteExposureFeatures( const DeviceSettings& devSettings, const SupportedWizardFeatures& features )
//-----------------------------------------------------------------------------
{
    pAcC_->exposureTime.write( devSettings.exposureTime );
    if( features.boAutoExposureSupport )
    {
        pAcC_->exposureAuto.writeS( string( devSettings.boAutoExposureEnabled ? "Continuous" : "Off" ) );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::WriteGainFeatures( const DeviceSettings& devSettings, const SupportedWizardFeatures& features )
//-----------------------------------------------------------------------------
{
    if( features.boAutoGainSupport )
    {
        if( pAnC_->gainSelector.readS() != string( "AnalogAll" ) )
        {
            pAnC_->gainSelector.writeS( "AnalogAll" );
        }
        pAnC_->gainAuto.writeS( string( devSettings.boAutoGainEnabled ? "Continuous" : "Off" ) );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupGenICam::WriteWhiteBalanceFeatures( const DeviceSettings& devSettings, const SupportedWizardFeatures& features )
//-----------------------------------------------------------------------------
{
    pAnC_->balanceRatioSelector.writeS( "Red" );
    pAnC_->balanceRatio.write( devSettings.whiteBalanceRed / 100. );
    pAnC_->balanceRatioSelector.writeS( "Blue" );
    pAnC_->balanceRatio.write( devSettings.whiteBalanceBlue / 100. );
    if( features.boAutoWhiteBalanceSupport )
    {
        pAnC_->balanceWhiteAuto.writeS( string( devSettings.boAutoWhiteBalanceEnabled ? "Continuous" : "Off" ) );
    }
}


//=============================================================================
//============== Implementation WizardQuickSetupDeviceSpecific ================
//=============================================================================
//-----------------------------------------------------------------------------
WizardQuickSetupDeviceSpecific::WizardQuickSetupDeviceSpecific( PropViewFrame* pParent, const wxString& title, bool boShowAtStartup ) :
    WizardQuickSetup( pParent, title, boShowAtStartup ), pCSBF_( 0 )
//-----------------------------------------------------------------------------
{

}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::CreateInterfaceLayoutSpecificControls( Device* pDev )
//-----------------------------------------------------------------------------
{
    pCSBF_ = new CameraSettingsBlueFOX( pDev );
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::DeleteInterfaceLayoutSpecificControls( void )
//-----------------------------------------------------------------------------
{
    DeleteElement( pCSBF_ );
}

//-----------------------------------------------------------------------------
double WizardQuickSetupDeviceSpecific::DoReadUnifiedBlackLevel( void ) const
//-----------------------------------------------------------------------------
{
    if( pIP_->gainOffsetKneeEnable.isValid() )
    {
        if( pIP_->gainOffsetKneeEnable.read() == bFalse )
        {
            pIP_->gainOffsetKneeEnable.write( bTrue );
        }
        return pIP_->gainOffsetKneeMasterOffset_pc.read();
    }
    return 0.;
}

//-----------------------------------------------------------------------------
double WizardQuickSetupDeviceSpecific::DoReadUnifiedGain( void ) const
//-----------------------------------------------------------------------------
{
    if ( pCSBF_->gain_dB.isValid() )
    {
        return pCSBF_->gain_dB.read();
    }
    return 0.;
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::DoWriteUnifiedBlackLevelData( double value )
//-----------------------------------------------------------------------------
{
    try
    {
        if( pIP_->gainOffsetKneeEnable.isValid() )
        {
            if( pIP_->gainOffsetKneeEnable.read() == bFalse )
            {
                pIP_->gainOffsetKneeEnable.write( bTrue );
            }
            pIP_->gainOffsetKneeMasterOffset_pc.write( value );
        }
    }
    catch( const ImpactAcquireException& e )
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to write blackLevel value:\n%s(%s)" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::DoWriteUnifiedGain( double value ) const
//-----------------------------------------------------------------------------
{
    if( pCSBF_->gain_dB.isValid() && pCSBF_->gain_dB.isVisible() )
    {
        pCSBF_->gain_dB.write( value );
    }
}
//-----------------------------------------------------------------------------
string WizardQuickSetupDeviceSpecific::GetPixelFormat( void ) const
//-----------------------------------------------------------------------------
{
    return ( pCSBF_->pixelFormat.isValid() ) ? pCSBF_->pixelFormat.readS() : string( "Mono8" );
}

//-----------------------------------------------------------------------------
double WizardQuickSetupDeviceSpecific::GetWhiteBalance( TWhiteBalanceChannel channel )
//-----------------------------------------------------------------------------
{
    //Driver WhiteBalance for mvBF2
    WhiteBalanceSettings& wbs = pIP_->getWBUserSetting( 0 );
    return ( channel == wbcRed ) ? wbs.redGain.read() : wbs.blueGain.read();
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::InitializeExposureParameters( DeviceSettings& devSettings, const SupportedWizardFeatures& features )
//-----------------------------------------------------------------------------
{
    if( features.boAutoExposureSupport )
    {
        AutoControlParameters ACP = pCSBF_->getAutoControlParameters();
        ACP.aoiMode.writeS( "Full" );
        pCSBF_->autoExposeControl.writeS( "On" );
        ACP.exposeUpperLimit_us.write( 200000. );
        ACP.desiredAverageGreyValue.write( 70 );
        devSettings.boAutoExposureEnabled = true;
    }
    else
    {
        SetExposureTime( 25000 );
        devSettings.boAutoExposureEnabled = false;
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::InitializeGainParameters( DeviceSettings& devSettings, const SupportedWizardFeatures& features )
//-----------------------------------------------------------------------------
{
    if( features.boAutoGainSupport )
    {
        pCSBF_->autoGainControl.writeS( "On" );
        devSettings.boAutoGainEnabled = true;
    }
    else
    {
        DoWriteUnifiedGain( 0. );
        devSettings.boAutoGainEnabled = false;
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::InitializeBlackLevelParameters( DeviceSettings&, const SupportedWizardFeatures& )
//-----------------------------------------------------------------------------
{
    pCSBF_->offsetAutoCalibration.write( aocOn );
    DoWriteUnifiedBlackLevelData( 0. );
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::InitializeWhiteBalanceParameters( DeviceSettings&, const SupportedWizardFeatures& )
//-----------------------------------------------------------------------------
{
    WhiteBalanceSettings& wbs = pIP_->getWBUserSetting( 0 );
    wbs.WBAoiMode.write( amFull );
    pIP_->whiteBalance.write( wbpUser1 );
    pIP_->whiteBalanceCalibration.write( wbcmNextFrame );
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::QueryInterfaceLayoutSpecificSettings( DeviceSettings& devSettings )
//-----------------------------------------------------------------------------
{
    if( pCSBF_->expose_us.isValid() )
    {
        devSettings.exposureTime = pCSBF_->expose_us.read();
    }
    if( pCSBF_->autoExposeControl.isValid() )
    {
        devSettings.boAutoExposureEnabled = pCSBF_->autoExposeControl.read() != aecOff;
    }

    if( pCSBF_->autoGainControl.isValid() )
    {
        devSettings.boAutoGainEnabled = pCSBF_->autoGainControl.read() != agcOff;
    }

    if( pCSBF_->offset_pc.isValid() )
    {
        devSettings.unifiedBlackLevel = DoReadUnifiedBlackLevel();
    }

    WhiteBalanceSettings& wbs = pIP_->getWBUserSetting( 0 );
    devSettings.whiteBalanceRed = wbs.redGain.read() * 100.;
    devSettings.whiteBalanceBlue = wbs.blueGain.read() * 100.;

    //No AutoWhitebalance for BF2
    devSettings.boAutoWhiteBalanceEnabled = false;
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::RestoreFactoryDefault( void )
//-----------------------------------------------------------------------------
{
    pCSBF_->restoreDefault();
    WhiteBalanceSettings& wbs = pIP_->getWBUserSetting( 0 );
    wbs.redGain.write( 1.0 );
    wbs.blueGain.write( 1.0 );
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::SelectColorPixelFormat( void )
//-----------------------------------------------------------------------------
{
    static const string m10 = string( "Mono10" );
    static const string m8 = string( "Mono8" );
    static const string defaultValue = string( "Auto" );

    vector<string> pixelFormatStrings;
    pCSBF_->pixelFormat.getTranslationDictStrings( pixelFormatStrings );
    const vector<string>::const_iterator itBEGIN = pixelFormatStrings.begin();
    const vector<string>::const_iterator itEND = pixelFormatStrings.end();
    if( find( itBEGIN, itEND, m10 ) != itEND )
    {
        pCSBF_->pixelFormat.writeS( m10 );
        currentSettings_[currentDeviceSerial_].imageFormatControlPixelFormat = m10;
    }
    else if( find( itBEGIN, itEND, m8 ) != itEND )
    {
        pCSBF_->pixelFormat.writeS( m8 );
        currentSettings_[currentDeviceSerial_].imageFormatControlPixelFormat = m8;
    }
    else
    {
        pCSBF_->pixelFormat.writeS( defaultValue );
        currentSettings_[currentDeviceSerial_].imageFormatControlPixelFormat = defaultValue;
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::SelectGreyscalePixelFormat( void )
//-----------------------------------------------------------------------------
{
    if( !featuresSupported_[currentDeviceSerial_].boColorOptionsSupport )
    {
        static const string m10 = string( "Mono10" );
        static const string m8 = string( "Mono8" );
        static const string defaultValue = string( "Auto" );

        vector<string> pixelFormatStrings;
        pCSBF_->pixelFormat.getTranslationDictStrings( pixelFormatStrings );
        vector<string>::iterator itEND = pixelFormatStrings.end();
        if( find( pixelFormatStrings.begin(), itEND, m10 ) != itEND )
        {
            pCSBF_->pixelFormat.writeS( m10 );
            currentSettings_[currentDeviceSerial_].imageFormatControlPixelFormat = m10;
        }
        else if( find( pixelFormatStrings.begin(), itEND, m8 ) != itEND )
        {
            pCSBF_->pixelFormat.writeS( m8 );
            currentSettings_[currentDeviceSerial_].imageFormatControlPixelFormat = m8;
        }
        else
        {
            pCSBF_->pixelFormat.writeS( defaultValue );
            currentSettings_[currentDeviceSerial_].imageFormatControlPixelFormat = defaultValue;
        }
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::SetupExposureControls( void )
//-----------------------------------------------------------------------------
{
    if( pCSBF_->expose_us.isValid() )
    {
        const double exposureMin = pCSBF_->expose_us.hasMinValue() ? pCSBF_->expose_us.getMinValue() : 1.;
        const double exposureMax = ( pCSBF_->expose_us.hasMaxValue() && ( pCSBF_->expose_us.getMaxValue() < 200000. ) ) ? pCSBF_->expose_us.getMaxValue() : 200000.;
        const double exposure = pCSBF_->expose_us.read();
        const bool boHasStepWidth = pCSBF_->expose_us.hasStepWidth();
        const double increment = boHasStepWidth ? static_cast<double>( pCSBF_->expose_us.getStepWidth() ) : 1.;
        DoSetupExposureControls( exposureMin, exposureMax, exposure, boHasStepWidth, increment );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::SetupGainControls( void )
//-----------------------------------------------------------------------------
{
    if( pCSBF_->gain_dB.isValid() )
    {
        const double gainUnifiedRangeMin = analogGainMin_ + digitalGainMin_;
        const double gainUnifiedRangeMax = analogGainMax_ + digitalGainMax_;
        const double gain = DoReadUnifiedGain();
        DoSetupGainControls( gainUnifiedRangeMin, gainUnifiedRangeMax, gain );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::SetupBlackLevelControls( void )
//-----------------------------------------------------------------------------
{
    if( pIP_->gainOffsetKneeEnable.isValid() )
    {
        const double blackLevelUnifiedRangeMin = -24.;
        const double blackLevelUnifiedRangeMax = 24.;
        const double blackLevel = DoReadUnifiedBlackLevel();
        DoSetupBlackLevelControls( blackLevelUnifiedRangeMin, blackLevelUnifiedRangeMax, blackLevel );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::SetupWhiteBalanceControls( void )
//-----------------------------------------------------------------------------
{
    if( featuresSupported_[currentDeviceSerial_].boColorOptionsSupport )
    {
        WhiteBalanceSettings& wbs = pIP_->getWBUserSetting( 0 );
        const double whiteBalanceRMin = 10.;
        const double whiteBalanceRMax = 500.;
        const double whiteBalanceR = wbs.redGain.read() * 100.0;
        const double whiteBalanceBMin = 10.;
        const double whiteBalanceBMax = 500.;
        const double whiteBalanceB = wbs.blueGain.read() * 100.0;
        DoSetupWhiteBalanceControls( whiteBalanceRMin, whiteBalanceRMax, whiteBalanceR, whiteBalanceBMin, whiteBalanceBMax, whiteBalanceB );
    }
    else
    {
        // This has to be done for aesthetic reasons. If a grayscale camera is opened, the whitebalance controls are
        // of course grayed out, however the last values (e.g. from the previous color camera) are still being shown
        DoSetupWhiteBalanceControls( 1., 500., 100., 1., 500., 100. );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::SetWhiteBalance( TWhiteBalanceChannel channel, double value )
//-----------------------------------------------------------------------------
{
    //WhiteBalance for mvBF2 has to be done via driver Properties
    try
    {
        WhiteBalanceSettings& wbs = pIP_->getWBUserSetting( 0 );
        if( pIP_->whiteBalance.read() != wbpUser1 )
        {
            pIP_->whiteBalance.write( wbpUser1 );
        }
        if( channel == wbcRed )
        {
            wbs.redGain.write( value );
        }
        else if( channel == wbcBlue )
        {
            wbs.blueGain.write( value );
        }
    }
    catch( const ImpactAcquireException& e )
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Failed to write white balance values:\n%s(%s)" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::SetAutoGain( bool boEnable )
//-----------------------------------------------------------------------------
{
    pCSBF_->autoGainControl.write( boEnable ? agcOn : agcOff );
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::SetupUnifiedGainData( void )
//-----------------------------------------------------------------------------
{
    if( pCSBF_->gain_dB.isValid() )
    {
        currentSettings_[currentDeviceSerial_].analogGainMax = pCSBF_->gain_dB.getMaxValue();
        currentSettings_[currentDeviceSerial_].analogGainMin = pCSBF_->gain_dB.getMinValue();
        currentSettings_[currentDeviceSerial_].digitalGainMax = 0;
        currentSettings_[currentDeviceSerial_].digitalGainMin = 0;
    }
    else
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Device has no 'Gain_dB' Property!" ) ) );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::SetupUnifiedBlackLevelData( void )
//-----------------------------------------------------------------------------
{
    if( pIP_->gainOffsetKneeEnable.isValid() )
    {
        if( pIP_->gainOffsetKneeEnable.read() == bFalse )
        {
            pIP_->gainOffsetKneeEnable.write( bTrue );
        }
        currentSettings_[currentDeviceSerial_].analogBlackLevelMax = pIP_->gainOffsetKneeMasterOffset_pc.getMaxValue();
        currentSettings_[currentDeviceSerial_].analogBlackLevelMin = pIP_->gainOffsetKneeMasterOffset_pc.getMinValue();
        currentSettings_[currentDeviceSerial_].digitalBlackLevelMax = 0.;
        currentSettings_[currentDeviceSerial_].digitalBlackLevelMin = 0.;
    }
    else
    {
        WriteQuickSetupWizardErrorMessage( wxString::Format( wxT( "Device has no 'gainOffsetKneeEnable' Property!" ) ) );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::WriteExposureFeatures( const DeviceSettings& devSettings, const SupportedWizardFeatures& features )
//-----------------------------------------------------------------------------
{
    pCSBF_->expose_us.write( devSettings.exposureTime );
    if( features.boAutoExposureSupport )
    {
        pCSBF_->autoExposeControl.write( devSettings.boAutoExposureEnabled ? aecOn : aecOff );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::WriteGainFeatures( const DeviceSettings& devSettings, const SupportedWizardFeatures& features )
//-----------------------------------------------------------------------------
{
    if( features.boAutoGainSupport )
    {
        pCSBF_->autoGainControl.write( devSettings.boAutoGainEnabled ? agcOn : agcOff );
    }
}

//-----------------------------------------------------------------------------
void WizardQuickSetupDeviceSpecific::WriteWhiteBalanceFeatures( const DeviceSettings& devSettings, const SupportedWizardFeatures& features )
//-----------------------------------------------------------------------------
{
    WhiteBalanceSettings& wbs = pIP_->getWBUserSetting( 0 );
    wbs.redGain.write( devSettings.whiteBalanceRed / 100. );
    wbs.blueGain.write( devSettings.whiteBalanceBlue / 100. );
    if( features.boAutoWhiteBalanceSupport )
    {
        //In case continuous autowhitebalancing is implemented:
        //pCSBF_->"balanceWhiteAuto-Property".writeS( string( devSettings.boAutoWhiteBalanceEnabled ? "Continuous" : "Off" ) );
    }
}
